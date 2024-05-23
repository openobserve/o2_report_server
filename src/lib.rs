// Copyright 2024 Zinc Labs Inc.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

pub mod config;
pub mod router;

use chromiumoxide::{browser::Browser, cdp::browser_protocol::page::PrintToPdfParams, Page};
use config::{get_chrome_launch_options, CONFIG};
use futures::StreamExt;
use lettre::{
    message::{header::ContentType, MultiPart, SinglePart},
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};
use serde::{Deserialize, Serialize};
use tokio::time::{sleep, Duration};

#[derive(Debug, Clone)]
pub struct SmtpConfig {
    pub from_email: String,
    pub reply_to: String,
    pub client: &'static AsyncSmtpTransport<Tokio1Executor>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EmailDetails {
    pub recepients: Vec<String>,
    pub title: String,
    pub name: String,
    pub message: String,
    pub dashb_url: String,
}

#[derive(Serialize, Debug, Deserialize, Clone)]
pub struct Report {
    pub dashboards: Vec<ReportDashboard>,
    pub email_details: EmailDetails,
}

#[derive(Serialize, Debug, Deserialize, Clone)]
pub struct ReportDashboard {
    pub dashboard: String,
    pub folder: String,
    pub tabs: Vec<String>,
    #[serde(default)]
    pub variables: Vec<ReportDashboardVariable>,
    /// The timerange of dashboard data.
    #[serde(default)]
    pub timerange: ReportTimerange,
}

#[derive(Serialize, Debug, Default, Deserialize, Clone)]
pub struct ReportDashboardVariable {
    pub key: String,
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

#[derive(Serialize, Debug, Default, Deserialize, Clone)]
pub enum ReportTimerangeType {
    #[default]
    #[serde(rename = "relative")]
    Relative,
    #[serde(rename = "absolute")]
    Absolute,
}

#[derive(Serialize, Debug, Deserialize, Clone)]
pub struct ReportTimerange {
    #[serde(rename = "type")]
    pub range_type: ReportTimerangeType,
    pub period: String, // 15m, 4M etc. For relative.
    pub from: i64,      // For absolute, in microseconds
    pub to: i64,        // For absolute, in microseconds
}

impl Default for ReportTimerange {
    fn default() -> Self {
        Self {
            range_type: ReportTimerangeType::default(),
            period: "1w".to_string(),
            from: 0,
            to: 0,
        }
    }
}

pub async fn generate_report(
    dashboard: &ReportDashboard,
    org_id: &str,
    user_id: &str,
    user_pass: &str,
    web_url: &str,
    timezone: &str,
) -> Result<(Vec<u8>, String), anyhow::Error> {
    let dashboard_id = &dashboard.dashboard;
    let folder_id = &dashboard.folder;

    let mut dashb_vars = "".to_string();
    for variable in dashboard.variables.iter() {
        dashb_vars = format!("{}&var-{}={}", dashb_vars, variable.key, variable.value);
    }

    if dashboard.tabs.is_empty() {
        return Err(anyhow::anyhow!("Atleast one tab is required"));
    }
    // Only one tab is supported for now
    let tab_id = &dashboard.tabs[0];

    log::info!("launching browser for dashboard {dashboard_id}");
    let (mut browser, mut handler) =
        Browser::launch(get_chrome_launch_options().await.clone()).await?;
    log::info!("browser launched");

    let handle = tokio::task::spawn(async move {
        while let Some(h) = handler.next().await {
            match h {
                Ok(_) => continue,
                Err(_) => break,
            }
        }
    });

    log::debug!("Navigating to web url: {}", &web_url);
    let page = browser
        .new_page(&format!("{web_url}/login?login_as_internal_user=true"))
        .await?;
    page.disable_log().await?;
    log::debug!("headless: new page created");

    page.find_element("input[type='email']")
        .await?
        .click()
        .await?
        .type_str(user_id)
        .await?;
    log::debug!("headless: email input filled");

    page.find_element("input[type='password']")
        .await?
        .click()
        .await?
        .type_str(user_pass)
        .await?
        .press_key("Enter")
        .await?;
    log::debug!("headless: password input filled");

    // Does not seem to work for single page client application
    page.wait_for_navigation().await?;
    sleep(Duration::from_secs(5)).await;

    let timerange = &dashboard.timerange;

    // dashboard link in the email should contain data of the same period as the report
    let (dashb_url, email_dashb_url) = match timerange.range_type {
        ReportTimerangeType::Relative => {
            let period = &timerange.period;
            let (time_duration, time_unit) = period.split_at(period.len() - 1);
            let dashb_url = format!(
                "{web_url}/dashboards/view?org_identifier={org_id}&dashboard={dashboard_id}&folder={folder_id}&tab={tab_id}&refresh=Off&period={period}&timezone={timezone}&var-Dynamic+filters=%255B%255D&print=true{dashb_vars}",
            );
            log::debug!("dashb_url for dashboard {folder_id}/{dashboard_id}: {dashb_url}");

            let time_duration: i64 = time_duration.parse()?;
            let end_time = chrono::Utc::now().timestamp_micros();
            let start_time = match time_unit {
                "m" => {
                    end_time
                        - chrono::Duration::try_minutes(time_duration)
                            .unwrap()
                            .num_microseconds()
                            .unwrap()
                }
                "h" => {
                    end_time
                        - chrono::Duration::try_hours(time_duration)
                            .unwrap()
                            .num_microseconds()
                            .unwrap()
                }
                "d" => {
                    end_time
                        - chrono::Duration::try_days(time_duration)
                            .unwrap()
                            .num_microseconds()
                            .unwrap()
                }
                "w" => {
                    end_time
                        - chrono::Duration::try_weeks(time_duration)
                            .unwrap()
                            .num_microseconds()
                            .unwrap()
                }
                _ => {
                    end_time
                        - chrono::Duration::try_days(30 * time_duration)
                            .unwrap()
                            .num_microseconds()
                            .unwrap()
                }
            };

            let email_dashb_url = format!(
                "{web_url}/dashboards/view?org_identifier={org_id}&dashboard={dashboard_id}&folder={folder_id}&tab={tab_id}&refresh=Off&from={start_time}&to={end_time}&timezone={timezone}&var-Dynamic+filters=%255B%255D&print=true{dashb_vars}",
            );
            (dashb_url, email_dashb_url)
        }
        ReportTimerangeType::Absolute => {
            let url = format!(
                "{web_url}/dashboards/view?org_identifier={org_id}&dashboard={dashboard_id}&folder={folder_id}&tab={tab_id}&refresh=Off&from={}&to={}&timezone={timezone}&var-Dynamic+filters=%255B%255D&print=true{dashb_vars}",
                &timerange.from, &timerange.to
            );
            log::debug!("dashb_url for dashboard {folder_id}/{dashboard_id}: {url}");

            (url.clone(), url)
        }
    };

    log::debug!("headless: going to dash url");
    // First navigate to the correct org
    page.goto(&format!("{web_url}/?org_identifier={org_id}"))
        .await?;
    page.wait_for_navigation().await?;
    sleep(Duration::from_secs(2)).await;
    log::debug!("headless: navigated to the org_id: {org_id}");

    page.goto(&dashb_url).await?;
    log::debug!("headless: going to dash url");

    // Wait for navigation does not really wait until it is fully loaded
    page.wait_for_navigation().await?;

    log::debug!("waiting for data to load for dashboard {dashboard_id}");

    // If the span element is not rendered yet, capture whatever is loaded till now
    if let Err(e) = wait_for_panel_data_load(&page).await {
        log::error!(
            "[REPORT] error occurred while finding the span element for dashboard {dashboard_id}:{e}"
        );
    } else {
        log::info!("[REPORT] all panel data loaded for report dashboard: {dashboard_id}");
    }

    if let Err(e) = page.find_element("main").await {
        browser.close().await?;
        handle.await?;
        return Err(anyhow::anyhow!(
            "[REPORT] main element not rendered yet for dashboard {dashboard_id}: {e}"
        ));
    }
    if let Err(e) = page.find_element("div.displayDiv").await {
        browser.close().await?;
        handle.await?;
        return Err(anyhow::anyhow!(
            "[REPORT] div.displayDiv element not rendered yet for dashboard {dashboard_id}: {e}"
        ));
    }

    // Last two elements loaded means atleast the metric components have loaded.
    // Convert the page into pdf
    let pdf_data = page
        .pdf(PrintToPdfParams {
            landscape: Some(true),
            ..Default::default()
        })
        .await?;

    browser.close().await?;
    handle.await?;
    log::debug!("done with headless browser");
    Ok((pdf_data, email_dashb_url))
}

/// Sends emails to the [`Report`] recepients. Currently only one pdf data is supported.
async fn send_email(
    pdf_data: &[u8],
    email_details: EmailDetails,
    config: SmtpConfig,
) -> Result<(), anyhow::Error> {
    let mut recepients = vec![];
    for recepient in &email_details.recepients {
        recepients.push(recepient);
    }

    let mut email = Message::builder()
        .from(config.from_email.parse()?)
        .subject(format!("Openobserve Report - {}", &email_details.title));

    for recepient in recepients {
        email = email.to(recepient.parse()?);
    }

    if !config.reply_to.is_empty() {
        email = email.reply_to(config.reply_to.parse()?);
    }

    let email = email
        .multipart(
            MultiPart::mixed()
                .singlepart(SinglePart::html(email_details.message))
                .singlepart(SinglePart::html(format!(
                    "<p><a href='{}' target='_blank'>Link to dashboard</a></p>",
                    email_details.dashb_url
                )))
                .singlepart(
                    // Only supports PDF for now, attach the PDF
                    lettre::message::Attachment::new(
                        email_details.title, // Attachment filename
                    )
                    .body(pdf_data.to_owned(), ContentType::parse("application/pdf")?),
                ),
        )
        .unwrap();

    // Send the email
    match config.client.send(email).await {
        Ok(_) => {
            log::info!(
                "email sent successfully for the report {}",
                &email_details.name
            );
            Ok(())
        }
        Err(e) => Err(anyhow::anyhow!("Error sending email: {e}")),
    }
}

pub async fn wait_for_panel_data_load(page: &Page) -> Result<(), anyhow::Error> {
    let start = std::time::Instant::now();
    let timeout = std::time::Duration::from_secs(CONFIG.chrome.chrome_sleep_secs.into());
    loop {
        if page
            .find_element("span#dashboardVariablesAndPanelsDataLoaded")
            .await
            .is_ok()
        {
            return Ok(());
        }

        if start.elapsed() >= timeout {
            return Err(anyhow::anyhow!(
                "span element indicator for data load not rendered yet"
            ));
        }

        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
