pub const ACCESS_TOKEN_COOKIE: &str = "access_token";
pub const REFRESH_TOKEN_COOKIE: &str = "refresh_token";

pub fn build_token_cookie(name: &str, value: &str, max_age_secs: i64, secure: bool) -> String {
    let mut cookie =
        format!("{name}={value}; HttpOnly; SameSite=Lax; Path=/; Max-Age={max_age_secs}");

    if secure {
        cookie.push_str("; Secure");
    }

    cookie
}

pub fn build_clear_cookie(name: &str, secure: bool) -> String {
    let mut cookie = format!("{name}=; HttpOnly; SameSite=Lax; Path=/; Max-Age=0");

    if secure {
        cookie.push_str("; Secure");
    }

    cookie
}
