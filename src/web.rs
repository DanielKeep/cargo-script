extern crate curl;
use curl::http;

/// Obtains the raw URL for GitHub and Gist URLs
fn generate_url(url: &str) -> Result<String, curl::ErrCode> {
    let mut output = String::from(url);
    if output.contains("gist.github.com") {
        let content = match get_html_response(url) {
            Ok(content) => content,
            Err(message) => { return Err(message); }
        };
        output = get_raw_gist_url(&content).unwrap();
    }
    if output.starts_with("github.com") {
        output = output.replace("github.com", "raw.githubusercontent.com");
    }
    Ok(output)
}

/// Obtains the raw Gist URL from the Gist HTML Body
fn get_raw_gist_url(html: &str) -> Option<String> {
    for line in html.lines() {
        if line.contains("/raw/") {
            let suffix = line.split("\"").nth(1).unwrap();
            return Some(String::from("https://gist.githubusercontent.com") + suffix);
        }
    }
    None
}

/// Returns the HTML body's response.
fn get_html_response(url: &str) -> Result<String, curl::ErrCode> {
    let response = match http::handle().get(url).exec() {
        Ok(response) => response,
        Err(message) => { return Err(message); }
    };
    Ok(String::from(match ::std::str::from_utf8(response.get_body()) {
        Ok(reply) => reply,
        Err(_) => { panic!("response is not a valid UTF8 string"); }
    }).replace("fn main()", ""))
}

/// Opens a URL and returns the source code found at the address.
pub fn download_script(url: &str) -> Result<String, curl::ErrCode> {
    match generate_url(&url) {
        Ok(url) => match get_html_response(&url) {
            Ok(code) => Ok(code),
            Err(message) => Err(message)
        },
        Err(message) => Err(message)
    }
}
