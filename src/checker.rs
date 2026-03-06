use reqwest::Client;

pub async fn check_site(client: &Client, url: &str, timeout: u64) -> (bool, String) {
    let final_url = if !url.starts_with("http://") && !url.starts_with("https://") {
        format!("https://{}", url)
    } else {
        url.to_string()
    };

    match client
        .get(&final_url)
        .timeout(std::time::Duration::from_secs(timeout))
        .send()
        .await
    {
        Ok(resp) => {
            let success = resp.status().is_success();
            let status = format!("{}", resp.status());
            if !success {
                println!("⚠️  [{}] returned status: {}", final_url, status);
            }
            (success, status)
        }
        Err(e) => {
            let status = if e.is_timeout() {
                "Timeout".to_string()
            } else if e.is_builder() {
                "Builder Error".to_string()
            } else if e.is_request() {
                "Request Error".to_string()
            } else if e.is_connect() {
                "Connection Error".to_string()
            } else {
                format!("Error: {}", e)
            };
            println!("❌  [{}] failed: {}", final_url, status);
            (false, status)
        }
    }
}
