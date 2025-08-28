use anyhow::{Result, Context, bail};
use crate::cli::ARGS;
use crate::http::AUTH_CLIENT;
use serde::{Deserialize, Serialize};
use serde_json::json;
pub static VALID_HOSTNAMES: &[&str] = &[
    "kemono.cr", "kemono.su", "kemono.st", "kemono.party",
    "coomer.cr", "coomer.su", "coomer.st", "coomer.party"
];

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LoginRequest {
    username: String,
    password: String,
}

fn validate_domain(domain: &str) -> Result<()> {
    if !VALID_HOSTNAMES.contains(&domain) {
        bail!(
            "Invalid domain '{}'. Credentials will only be sent to official kemono/coomer domains: {}",
            domain,
            VALID_HOSTNAMES.join(", ")
        );
    }
    Ok(())
}


pub async fn authenticate(domains: &[&str]) -> Result<()> {
    if let (Some(username), Some(password)) = (&ARGS.username, &ARGS.password) {
        login_with_credentials(username, password, domains).await?;
    }
    
    Ok(())
}

async fn login_with_credentials(username: &str, password: &str, domains: &[&str]) -> Result<()> {
    eprintln!("Authenticating with username and password...");
    
    // Only allow a single unique domain
    let unique_domains: std::collections::HashSet<_> = domains.iter().collect();
    
    if unique_domains.len() > 1 {
        let domain_list: Vec<String> = unique_domains.iter().map(|s| s.to_string()).collect();
        bail!("Multiple domains found in URLs: {}. Authentication only supports a single domain per session.", 
              domain_list.join(", "));
    }
    
    if unique_domains.is_empty() {
        bail!("No valid domains found for authentication");
    }
    
    let domain = unique_domains.iter().next().unwrap();
    
    // Validate domain before sending credentials
    validate_domain(domain)?;
    
    let login_url = format!("https://{}/api/v1/authentication/login", domain);
    
    let login_data = json!({
        "username": username,
        "password": password
    });
    
    let response = AUTH_CLIENT
        .post(&login_url)
        .json(&login_data)
        .send()
        .await
        .context(format!("Failed to send login request to {}", domain))?;
    
    if !response.status().is_success() {
        // Sanitize error message - only show generic info
        let error_msg = match response.status().as_u16() {
            401 => "Invalid credentials",
            429 => "Too many login attempts. Please try again later",
            500..=599 => "Server error. Please try again later",
            _ => "Authentication failed",
        };
        
        bail!("Login failed for {}: {}", domain, error_msg);
    }
    
    eprintln!("Successfully authenticated with {}!", domain);
    
    Ok(())
}