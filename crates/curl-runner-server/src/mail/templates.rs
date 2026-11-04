//! Email copy. Short sentences, no em dashes.

use super::Mail;

/// Subject, plain text and HTML for `mail`.
pub fn render(mail: &Mail) -> (String, String, String) {
    match mail {
        Mail::SignInCode { code, link, .. } => (
            format!("Your curl-runner sign-in code: {code}"),
            format!(
                "Your curl-runner sign-in code is {code}.\n\nOr open this link:\n{link}\n\n\
                 It expires in 10 minutes. If you didn't ask for it, ignore this email.\n"
            ),
            html(&[
                &format!(
                    "Your curl-runner sign-in code is <strong>{}</strong>.",
                    esc(code)
                ),
                &format!("Or <a href=\"{}\">sign in with this link</a>.", esc(link)),
                "It expires in 10 minutes. If you didn't ask for it, ignore this email.",
            ]),
        ),
        Mail::MemberInvite {
            team_name,
            inviter,
            link,
            ..
        } => (
            format!("{inviter} invited you to {team_name} on curl-runner"),
            format!(
                "{inviter} invited you to join {team_name} on curl-runner.\n\n\
                 Accept the invite:\n{link}\n\nThe invite expires in 7 days.\n"
            ),
            html(&[
                &format!(
                    "{} invited you to join {} on curl-runner.",
                    esc(inviter),
                    esc(team_name)
                ),
                &format!("<a href=\"{}\">Accept the invite</a>.", esc(link)),
                "The invite expires in 7 days.",
            ]),
        ),
        Mail::CustomerInvite {
            team_name,
            collection_name,
            link,
            ..
        } => (
            format!("{team_name} shared {collection_name} with you"),
            format!(
                "{team_name} shared the collection {collection_name} with you.\n\n\
                 Open it:\n{link}\n"
            ),
            html(&[
                &format!(
                    "{} shared the collection {} with you.",
                    esc(team_name),
                    esc(collection_name)
                ),
                &format!("<a href=\"{}\">Open it</a>.", esc(link)),
            ]),
        ),
    }
}

fn html(paragraphs: &[&str]) -> String {
    let body: String = paragraphs.iter().map(|p| format!("<p>{p}</p>")).collect();
    format!("<!doctype html><html><body>{body}</body></html>")
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sign_in() -> Mail {
        Mail::SignInCode {
            to: "ada@example.com".into(),
            code: "123456".into(),
            link: "https://cloud.example.com/auth/magic?token=abc".into(),
        }
    }

    #[test]
    fn sign_in_template_has_code_and_no_em_dash() {
        let (subject, text, html) = render(&sign_in());
        assert_eq!(subject, "Your curl-runner sign-in code: 123456");
        assert!(text.contains("123456"));
        assert!(text.contains("https://cloud.example.com/auth/magic?token=abc"));
        assert!(
            text.contains("It expires in 10 minutes. If you didn't ask for it, ignore this email.")
        );
        assert!(html.contains("123456"));
        for s in [&subject, &text, &html] {
            assert!(!s.contains('\u{2014}'));
        }
    }

    #[test]
    fn invite_subjects_match_copy() {
        let (s, _, _) = render(&Mail::MemberInvite {
            to: "b@example.com".into(),
            team_name: "Acme".into(),
            inviter: "Ada".into(),
            link: "https://x.example.com/i".into(),
        });
        assert_eq!(s, "Ada invited you to Acme on curl-runner");
        let (s, text, html) = render(&Mail::CustomerInvite {
            to: "b@example.com".into(),
            team_name: "Acme".into(),
            collection_name: "payments-api".into(),
            link: "https://x.example.com/c".into(),
        });
        assert_eq!(s, "Acme shared payments-api with you");
        for s in [&text, &html] {
            assert!(!s.contains('\u{2014}'));
        }
    }

    #[test]
    fn html_escapes_user_text() {
        let (_, _, html) = render(&Mail::MemberInvite {
            to: "b@example.com".into(),
            team_name: "<b>Acme</b>".into(),
            inviter: "A & B".into(),
            link: "https://x.example.com/i?a=1&b=\"2\"".into(),
        });
        assert!(html.contains("&lt;b&gt;Acme&lt;/b&gt;"));
        assert!(html.contains("A &amp; B"));
        assert!(html.contains("a=1&amp;b=&quot;2&quot;"));
        assert!(!html.contains("<b>"));
    }
}
