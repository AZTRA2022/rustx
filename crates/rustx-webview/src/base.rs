//! What every window starts with: Inter as the default font, and the colours
//! the default screens (system pages, not-found, error) are drawn in.
//!
//! The theme is Next.js's: black on white, white on black by the system's
//! preference, hairline borders, a black-or-white primary button. It is
//! written as CSS variables (`--rx-bg`, `--rx-fg`, `--rx-muted`,
//! `--rx-border`, `--rx-accent`), so an application's own pages may use them
//! and a `system/*.rsx` of its own looks the same as the defaults.
//!
//! Only `html` gets the font: a page that names its own on `body` or below
//! keeps it. Inter is the variable font, five subsets (SIL Open Font License, see
//! `assets/Inter-OFL.txt`), carried in the binary as a `data:` URI so no
//! request is made and the window works offline.

/// Inter's subsets with the ranges Fontsource publishes for them: the browser
/// fetches (here: decodes) only the ones a page's text needs.
const SUBSETS: [(&[u8], &str); 5] = [
    (
        include_bytes!("../assets/inter-latin-wght-normal.woff2"),
        "U+0000-00FF,U+0131,U+0152-0153,U+02BB-02BC,U+02C6,U+02DA,U+02DC,U+0304,U+0308,U+0329,U+2000-206F,U+20AC,U+2122,U+2191,U+2193,U+2212,U+2215,U+FEFF,U+FFFD",
    ),
    (
        include_bytes!("../assets/inter-latin-ext-wght-normal.woff2"),
        "U+0100-02BA,U+02BD-02C5,U+02C7-02CC,U+02CE-02D7,U+02DD-02FF,U+0304,U+0308,U+0329,U+1D00-1DBF,U+1E00-1E9F,U+1EF2-1EFF,U+2020,U+20A0-20AB,U+20AD-20C0,U+2113,U+2C60-2C7F,U+A720-A7FF",
    ),
    (
        include_bytes!("../assets/inter-cyrillic-wght-normal.woff2"),
        "U+0301,U+0400-045F,U+0490-0491,U+04B0-04B1,U+2116",
    ),
    (
        include_bytes!("../assets/inter-greek-wght-normal.woff2"),
        "U+0370-0377,U+037A-037F,U+0384-038A,U+038C,U+038E-03A1,U+03A3-03FF",
    ),
    (
        include_bytes!("../assets/inter-vietnamese-wght-normal.woff2"),
        "U+0102-0103,U+0110-0111,U+0128-0129,U+0168-0169,U+01A0-01A1,U+01AF-01B0,U+0300-0301,U+0303-0304,U+0308-0309,U+0323,U+0329,U+1EA0-1EF9,U+20AB",
    ),
];

fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = (u32::from(chunk[0]) << 16)
            | (u32::from(*chunk.get(1).unwrap_or(&0)) << 8)
            | u32::from(*chunk.get(2).unwrap_or(&0));
        for (i, shift) in [18, 12, 6, 0].into_iter().enumerate() {
            if i <= chunk.len() {
                out.push(ALPHABET[((n >> shift) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// The stylesheet.
///
/// Everything is in `@layer rustx-base`: a rule an application writes, in
/// whatever order its stylesheets come, is outside every layer and so always
/// wins over these. The framework's look is a default, never an override.
pub fn css() -> String {
    let mut faces = String::new();
    for (bytes, range) in SUBSETS {
        faces.push_str(&format!(
            "@font-face{{font-family:Inter;font-style:normal;font-weight:100 900;font-display:swap;             src:url(data:font/woff2;base64,{}) format('woff2');unicode-range:{range}}}",
            base64(bytes)
        ));
    }
    format!(
        "@layer rustx-base{{{faces}         :root{{--rx-bg:#fff;--rx-fg:#000;--rx-muted:#666;--rx-border:#eaeaea;--rx-accent:#0070f3;--rx-danger:#c62a2f}}         @media (prefers-color-scheme:dark){{:root{{--rx-bg:#000;--rx-fg:#fff;--rx-muted:#a1a1a1;--rx-border:#333;--rx-accent:#3291ff;--rx-danger:#ff6369}}}}         :root[data-rx-theme=light]{{--rx-bg:#fff;--rx-fg:#000;--rx-muted:#666;--rx-border:#eaeaea;--rx-accent:#0070f3;--rx-danger:#c62a2f}}         :root[data-rx-theme=dark]{{--rx-bg:#000;--rx-fg:#fff;--rx-muted:#a1a1a1;--rx-border:#333;--rx-accent:#3291ff;--rx-danger:#ff6369}}         html{{font-family:Inter,system-ui,\"Segoe UI\",Roboto,Helvetica,Arial,sans-serif;         -webkit-font-smoothing:antialiased;text-rendering:optimizeLegibility}}}}"
    )
}

/// The script that puts it in every page, before the page's own. `RUSTX_THEME`
/// (`light` or `dark`) forces a theme, for looking at both without changing
/// the system's.
pub fn script() -> String {
    let theme = std::env::var("RUSTX_THEME").unwrap_or_default();
    let theme = if matches!(theme.as_str(), "light" | "dark") {
        theme
    } else {
        String::new()
    };
    format!(
        "(function(){{var put=function(){{var s=document.createElement('style');s.id='rustx-base';         s.textContent={};(document.head||document.documentElement).prepend(s);         if({theme:?})document.documentElement.setAttribute('data-rx-theme',{theme:?})}};         if(document.documentElement)put();         else document.addEventListener('readystatechange',put,{{once:true}})}})();",
        serde_json::json!(css())
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_is_the_standard_one() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn the_font_is_a_woff2_and_the_stylesheet_names_it() {
        for (bytes, _) in SUBSETS {
            assert_eq!(&bytes[..4], b"wOF2");
        }
        let css = css();
        assert!(
            css.contains("font-family:Inter")
                && css.contains("--rx-bg")
                && css.starts_with("@layer rustx-base")
        );
    }
}
