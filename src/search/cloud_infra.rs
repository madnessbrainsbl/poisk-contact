use crate::models::CloudInfraProfile;

pub fn detect_cloud_infra(headers_str: &str, body: &str) -> CloudInfraProfile {
    let lower_headers = headers_str.to_lowercase();
    let lower_body = body.to_lowercase();

    let mut profile = CloudInfraProfile::default();
    let mut markers = Vec::new();

    // 1. Cloudflare
    if lower_headers.contains("cf-ray")
        || lower_headers.contains("server: cloudflare")
        || lower_headers.contains("cf-cache-status")
        || lower_body.contains("cdnjs.cloudflare.com")
    {
        markers.push("Cloudflare Edge Network".to_string());
        profile.cdn = Some("Cloudflare".to_string());
        profile.provider = "Cloudflare".to_string();
        profile.is_cloud_hosted = true;
    }

    // 2. Amazon Web Services (AWS)
    if lower_headers.contains("x-amz-cf-id")
        || lower_headers.contains("x-amz-request-id")
        || lower_headers.contains("awselb")
        || lower_headers.contains("cloudfront")
        || lower_body.contains("amazonaws.com")
        || lower_body.contains("cloudfront.net")
    {
        markers.push("Amazon Web Services (AWS)".to_string());
        if profile.provider.is_empty() {
            profile.provider = "Amazon Web Services (AWS)".to_string();
        }
        if lower_headers.contains("cloudfront") || lower_headers.contains("x-amz-cf-id") {
            profile.cdn = Some("Amazon CloudFront".to_string());
        }
        profile.is_cloud_hosted = true;
    }

    // 3. Google Cloud Platform (GCP)
    if lower_headers.contains("x-goog-")
        || lower_headers.contains("server: gws")
        || lower_headers.contains("server: ghs")
        || lower_body.contains("storage.googleapis.com")
    {
        markers.push("Google Cloud Platform (GCP)".to_string());
        if profile.provider.is_empty() {
            profile.provider = "Google Cloud Platform (GCP)".to_string();
        }
        profile.is_cloud_hosted = true;
    }

    // 4. Microsoft Azure
    if lower_headers.contains("x-azure-ref")
        || lower_headers.contains("azureedge.net")
        || lower_body.contains("windows.net")
        || lower_headers.contains("x-ms-")
    {
        markers.push("Microsoft Azure".to_string());
        if profile.provider.is_empty() {
            profile.provider = "Microsoft Azure".to_string();
        }
        profile.is_cloud_hosted = true;
    }

    // 5. Regional & European Providers (Selectel, Hetzner, DigitalOcean)
    if lower_headers.contains("selectel") || lower_body.contains("selcdn.ru") {
        markers.push("Selectel Cloud".to_string());
        if profile.provider.is_empty() {
            profile.provider = "Selectel Cloud".to_string();
        }
        profile.is_cloud_hosted = true;
    }

    if lower_headers.contains("hetzner") || lower_headers.contains("your-server.de") {
        markers.push("Hetzner Online".to_string());
        if profile.provider.is_empty() {
            profile.provider = "Hetzner Online".to_string();
        }
        profile.is_cloud_hosted = true;
    }

    if lower_headers.contains("digitalocean") || lower_body.contains("digitaloceanspaces.com") {
        markers.push("DigitalOcean".to_string());
        if profile.provider.is_empty() {
            profile.provider = "DigitalOcean".to_string();
        }
        profile.is_cloud_hosted = true;
    }

    if profile.provider.is_empty() {
        profile.provider = "Dedicated / On-Premise".to_string();
    }

    profile.detected_markers = markers;
    profile
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cloudflare_detection() {
        let headers = "Server: cloudflare\r\nCF-RAY: 83b4c19a184e4f21-DME\r\nCF-Cache-Status: HIT";
        let body = "<html><body>Hello</body></html>";
        let prof = detect_cloud_infra(headers, body);
        assert_eq!(prof.provider, "Cloudflare");
        assert!(prof.is_cloud_hosted);
        assert_eq!(prof.cdn.as_deref(), Some("Cloudflare"));
    }

    #[test]
    fn test_aws_detection() {
        let headers = "Server: awselb/2.0\r\nx-amz-cf-id: aBcDeF12345==";
        let body =
            "<html><body><img src=\"https://mybucket.s3.amazonaws.com/pic.png\"/></body></html>";
        let prof = detect_cloud_infra(headers, body);
        assert_eq!(prof.provider, "Amazon Web Services (AWS)");
        assert!(prof.is_cloud_hosted);
        assert_eq!(prof.cdn.as_deref(), Some("Amazon CloudFront"));
    }

    #[test]
    fn test_azure_detection() {
        let headers = "Server: Microsoft-IIS/10.0\r\nX-Azure-Ref: 0aBcDeF==";
        let body = "<html><body>Azure service</body></html>";
        let prof = detect_cloud_infra(headers, body);
        assert_eq!(prof.provider, "Microsoft Azure");
        assert!(prof.is_cloud_hosted);
    }
}
