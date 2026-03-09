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
            let (short_status, full_status) = if e.is_timeout() {
                ("Timeout", format!("Timeout: {}", e))
            } else if e.is_builder() {
                ("Builder Error", format!("Builder Error: {}", e))
            } else if e.is_request() {
                ("Request Error", format!("Request Error: {}", e))
            } else if e.is_connect() {
                ("Connection Error", format!("Connection Error: {}", e))
            } else {
                ("Error", format!("Error: {}", e))
            };
            println!("❌  [{}] failed: {}", final_url, full_status);
            (false, short_status.to_string())
        }
    }
}
