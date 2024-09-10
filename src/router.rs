use actix_web::{get, http::StatusCode, put, web, HttpRequest, HttpResponse as ActixHttpResponse};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Error;

use crate::{
    config::{CONFIG, SMTP_CLIENT},
    Report, ReportType,
};

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
}

#[get("/healthz")]
pub async fn healthz() -> Result<ActixHttpResponse, Error> {
    Ok(ActixHttpResponse::Ok().body("Server up and running"))
}

#[put("/{org_id}/reports/{name}/send")]
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

    let report_type = if report.email_details.recepients.is_empty() {
        ReportType::Cache
    } else {
        ReportType::PDF
    };
    let (pdf_data, email_dashboard_url) = match crate::generate_report(
        &report.dashboards[0],
        &org_id,
        &CONFIG.auth.user_email,
        &CONFIG.auth.user_password,
        &report.email_details.dashb_url,
        timezone,
        report_type.clone(),
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
        &pdf_data,
        crate::EmailDetails {
            dashb_url: email_dashboard_url,
            ..report.email_details
        },
        crate::SmtpConfig {
            from_email: CONFIG.smtp.smtp_from_email.to_string(),
            reply_to: CONFIG.smtp.smtp_reply_to.to_string(),
            client: &SMTP_CLIENT,
        },
    )
    .await
    {
        Ok(_) => Ok(ActixHttpResponse::Ok().json(HttpResponse::success(format!(
            "report sent to emails successfully"
        )))),
        Err(e) => {
            log::error!("Error sending emails to recepients: {e}");
            Ok(ActixHttpResponse::InternalServerError()
                .json(HttpResponse::internal_server_error(e.to_string())))
        }
    }
}
