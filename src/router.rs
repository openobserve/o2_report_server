// Copyright 2025 OpenObserve Inc.
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

use crate::EmailAttachmentType::Inline;
use crate::{
    config::{CONFIG, SMTP_CLIENT},
    Report, ReportType,
};
use actix_web::{
    dev::Service, get, http::StatusCode, web, HttpRequest, HttpResponse as ActixHttpResponse,
};
use futures::future::{ready, Either};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Error;

pub const REPORT_SECRET_HEADER: &str = "x-o2-report-secret";

/// HTTP response
/// code 200 is success
/// code 400 is error
/// code 404 is not found
/// code 500 is internal server error
/// code 503 is service unavailable
/// code >= 1000 is custom error code
/// message is the message or error message
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HttpResponse {
    pub code: u16,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
}

impl HttpResponse {
    pub fn internal_server_error(e: String) -> Self {
        Self {
            code: StatusCode::INTERNAL_SERVER_ERROR.into(),
            message: e,
            error_detail: None,
            trace_id: None,
        }
    }

    pub fn success(msg: String) -> Self {
        Self {
            code: StatusCode::OK.into(),
            message: msg,
            error_detail: None,
            trace_id: None,
        }
    }

    pub fn new(msg: String, status_code: u16) -> Self {
        Self {
            code: status_code,
            message: msg,
            error_detail: None,
            trace_id: None,
        }
    }
}

/// Shared secret the main app must send in [`REPORT_SECRET_HEADER`] to trigger a report.
pub struct ReportSecret(pub String);

#[get("/healthz")]
pub async fn healthz() -> Result<ActixHttpResponse, Error> {
    Ok(ActixHttpResponse::Ok().body("Server up and running"))
}

pub async fn send_report(
    report: web::Json<Report>,
    path: web::Path<(String, String)>,
    req: HttpRequest,
) -> Result<ActixHttpResponse, Error> {
    let report = report.into_inner();
    let (org_id, report_name) = path.into_inner();
    let query = web::Query::<HashMap<String, String>>::from_query(req.query_string()).unwrap();
    let timezone = match query.get("timezone") {
        Some(v) => v,
        None => "Europe/London",
    };

    // ensure a dashboard was provided and if not raise a helpful error with a 400
    if report.dashboards.len() == 0 {
        log::error!("At least 1 dashboard must be provided when sending a report");
        return Ok(
            ActixHttpResponse::build(StatusCode::BAD_REQUEST).json(HttpResponse::new(
                "At least 1 dashboard must be provided when sending a report".to_string(),
                StatusCode::BAD_REQUEST.into(),
            )),
        );
    }

    // Since only 1 dashboard is supported currently per report, grab the first one
    let dashboard_for_report = report.dashboards[0].clone();
    let report_type = if report.email_details.recipients.is_empty() {
        ReportType::Cache
    } else {
        dashboard_for_report.report_type.clone()
    };

    // If inline attachment was desired but not a PDF, raise an exception since most mail servers
    // will only let you embed simple images.
    if report_type == ReportType::PDF && dashboard_for_report.email_attachment_type == Inline {
        log::warn!("Inline PDF attachments are not allowed. Report: {org_id}/{report_name}");
        return Ok(
            ActixHttpResponse::build(StatusCode::CONFLICT).json(HttpResponse::new(
                "Most email servers do not support inline PDF attachments, \
                for inline attachments please use a PNG."
                    .to_string(),
                StatusCode::CONFLICT.into(),
            )),
        );
    }

    let image_preview = report.email_details.image_preview;

    let (attachment_data, email_dashboard_url, preview_image) = match crate::generate_report(
        &dashboard_for_report,
        &org_id,
        &report_name,
        &CONFIG.auth.user_email,
        &CONFIG.auth.user_password,
        &report.email_details.dashb_url,
        timezone,
        report_type.clone(),
        image_preview,
    )
    .await
    {
        Ok(res) => res,
        Err(e) => {
            log::error!("Error generating pdf for report {org_id}/{report_name}: {e}");
            return Ok(ActixHttpResponse::InternalServerError()
                .json(HttpResponse::internal_server_error(e.to_string())));
        }
    };

    if report_type == ReportType::Cache {
        log::info!("Dashboard data cached by report {report_name}");
        return Ok(ActixHttpResponse::Ok().json(HttpResponse::success(format!(
            "dashboard data cached by report {report_name}"
        ))));
    }

    match crate::send_email(
        &attachment_data,
        report.dashboards[0].report_type.clone(),
        report.dashboards[0].email_attachment_type.clone(),
        crate::EmailDetails {
            dashb_url: email_dashboard_url,
            ..report.email_details
        },
        crate::SmtpConfig {
            from_email: CONFIG.smtp.smtp_from_email.to_string(),
            reply_to: CONFIG.smtp.smtp_reply_to.to_string(),
            client: &SMTP_CLIENT,
        },
        preview_image,
    )
    .await
    {
        Ok(_) => Ok(ActixHttpResponse::Ok().json(HttpResponse::success(
            "report sent to emails successfully".to_string(),
        ))),
        Err(e) => {
            log::error!("Error sending emails to recepients: {e}");
            Ok(ActixHttpResponse::InternalServerError()
                .json(HttpResponse::internal_server_error(e.to_string())))
        }
    }
}

/// Mounts `/api`; only the send route requires [`REPORT_SECRET_HEADER`] to match [`ReportSecret`].
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api").service(healthz).service(
            web::resource("/{org_id}/reports/{name}/send")
                .wrap_fn(|req, srv| {
                    let authorized = secret_matches(
                        req.app_data::<web::Data<ReportSecret>>()
                            .map_or("", |secret| secret.0.as_str()),
                        req.headers()
                            .get(REPORT_SECRET_HEADER)
                            .map(|value| value.as_bytes()),
                    );
                    if authorized {
                        Either::Left(srv.call(req))
                    } else {
                        Either::Right(ready(Ok(req.into_response(
                            ActixHttpResponse::Unauthorized().json(HttpResponse::new(
                                "invalid report server secret".to_string(),
                                StatusCode::UNAUTHORIZED.into(),
                            )),
                        ))))
                    }
                })
                .route(web::put().to(send_report)),
        ),
    );
}

/// Compares in constant time and treats an empty configured secret as matching nothing. [pure]
fn secret_matches(configured: &str, provided: Option<&[u8]>) -> bool {
    let configured = configured.as_bytes();
    let Some(provided) = provided else {
        return false;
    };
    if configured.is_empty() || configured.len() != provided.len() {
        return false;
    }
    configured
        .iter()
        .zip(provided)
        .fold(0u8, |diff, (a, b)| diff | (a ^ b))
        == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App};

    const BODY: &str = r#"{"dashboards":[],"email_details":{"recipients":[],"title":"","name":"","message":"","dashb_url":""}}"#;

    async fn send(configured: &str, header: Option<&str>) -> StatusCode {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(ReportSecret(configured.to_string())))
                .configure(configure),
        )
        .await;
        let mut req = test::TestRequest::put()
            .uri("/api/org1/reports/r1/send")
            .insert_header(("content-type", "application/json"))
            .set_payload(BODY);
        if let Some(value) = header {
            req = req.insert_header((REPORT_SECRET_HEADER, value));
        }
        test::call_service(&app, req.to_request()).await.status()
    }

    #[actix_web::test]
    async fn send_route_requires_matching_secret() {
        let cases = [
            (
                "wrong secret",
                "s3cret",
                Some("nope"),
                StatusCode::UNAUTHORIZED,
            ),
            ("missing header", "s3cret", None, StatusCode::UNAUTHORIZED),
            (
                "empty configured secret",
                "",
                Some(""),
                StatusCode::UNAUTHORIZED,
            ),
            (
                "right secret",
                "s3cret",
                Some("s3cret"),
                StatusCode::BAD_REQUEST,
            ),
        ];
        for (name, configured, header, expected) in cases {
            assert_eq!(send(configured, header).await, expected, "{name}");
        }
    }

    #[actix_web::test]
    async fn healthz_needs_no_secret() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(ReportSecret("s3cret".to_string())))
                .configure(configure),
        )
        .await;
        let req = test::TestRequest::get().uri("/api/healthz").to_request();
        assert_eq!(test::call_service(&app, req).await.status(), StatusCode::OK);
    }
}
