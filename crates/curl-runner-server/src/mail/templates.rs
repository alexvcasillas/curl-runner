//! Email copy. Short sentences, no em dashes.

use super::Mail;

const SIGN_IN_FOOTER: &str =
    "It expires in 10 minutes. If you didn't ask for it, ignore this email.";

/// Subject, plain text and HTML for `mail`. The subject is one line: user
/// text in it (team and collection names) cannot add a header.
pub fn render(mail: &Mail) -> (String, String, String) {
    let (subject, text, html) = render_parts(mail);
    (one_line(&subject), text, html)
}

/// `s` with CR, LF and other control characters replaced by spaces.
fn one_line(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect()
}

fn render_parts(mail: &Mail) -> (String, String, String) {
    match mail {
        Mail::SignInCode {
            code,
            link: Some(link),
            ..
        } => (
            format!("Your curl-runner sign-in code: {code}"),
            format!(
                "Your curl-runner sign-in code is {code}.\n\nOr open this link:\n{link}\n\n\
                 {SIGN_IN_FOOTER}\n"
            ),
            layout(
                "Your curl-runner sign-in code is:",
                &[
                    Block::Code(esc(code)),
                    Block::Button("Sign in with this link", esc(link)),
                    Block::Fine(SIGN_IN_FOOTER.into()),
                ],
            ),
        ),
        Mail::SignInCode {
            code, link: None, ..
        } => (
            format!("Your curl-runner sign-in code: {code}"),
            format!(
                "Your curl-runner sign-in code is {code}.\n\n\
                 Enter it in your terminal. {SIGN_IN_FOOTER}\n"
            ),
            layout(
                "Your curl-runner sign-in code is:",
                &[
                    Block::Code(esc(code)),
                    Block::Text("Enter it in your terminal.".into()),
                    Block::Fine(SIGN_IN_FOOTER.into()),
                ],
            ),
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
                 {MEMBER_ROLE}\n\nAccept the invite:\n{link}\n\n{INVITE_FOOTER}\n"
            ),
            layout(
                &format!(
                    "{} invited you to join <b>{}</b> on curl-runner.",
                    esc(inviter),
                    esc(team_name)
                ),
                &[
                    Block::Muted(MEMBER_ROLE.into()),
                    Block::Button("Accept the invite", esc(link)),
                    Block::Fine(INVITE_FOOTER.into()),
                ],
            ),
        ),
        Mail::CustomerInvite {
            to,
            team_name,
            collection_name,
            link,
        } => {
            let how = format!(
                "Open it in curl-runner studio, or pull it with the CLI. \
                 Sign in with {to} when it asks."
            );
            (
                format!("{team_name} shared {collection_name} with you"),
                format!(
                    "{team_name} shared the collection {collection_name} with you.\n\n\
                 {how}\n\nOpen it:\n{link}\n\n\
                 You're getting this because someone at {team_name} added your email.\n"
                ),
                layout(
                    &format!(
                        "{} shared the collection <b>{}</b> with you.",
                        esc(team_name),
                        esc(collection_name)
                    ),
                    &[
                        Block::Muted(esc(&how)),
                        Block::Button("Open it", esc(link)),
                        Block::Fine(format!(
                            "You\u{2019}re getting this because someone at {} added your email.",
                            esc(team_name)
                        )),
                    ],
                ),
            )
        }
        Mail::PaymentFailed {
            team_name,
            slug,
            deadline,
            app_url,
            ..
        } => {
            let date = deadline.format("%b %-d").to_string();
            let link = format!("{app_url}/t/{slug}/billing");
            (
                format!("We couldn't take {team_name}'s payment"),
                format!(
                    "We couldn't take the last payment for {team_name}. Update your card by \
                     {date} to keep the Team plan.\n\nStripe will try the card again. After \
                     {date}, {team_name} moves to Free and nothing is deleted.\n\n\
                     Update card:\n{link}\n\nReceipts and invoices come from Stripe.\n"
                ),
                layout(
                    &format!("We couldn't take the last payment for {}.", esc(team_name)),
                    &[
                        Block::Text("Stripe will try the card again.".into()),
                        Block::Callout(format!(
                            "Update your card by <b>{}</b> to keep the Team plan. After that, \
                             {} moves to Free and nothing is deleted.",
                            esc(&date),
                            esc(team_name)
                        )),
                        Block::Button("Update card", esc(&link)),
                        Block::Fine("Receipts and invoices come from Stripe.".into()),
                    ],
                ),
            )
        }
        Mail::TestEmail { .. } => (
            "curl-runner test email".into(),
            "This is a test email from your curl-runner server.\n\n\
             If you can read it, sending works. Nothing else to do.\n"
                .into(),
            layout(
                "This is a test email from your curl-runner server.",
                &[Block::Text(
                    "If you can read it, sending works. Nothing else to do.".into(),
                )],
            ),
        ),
        Mail::PlanChanged {
            team_name,
            slug,
            app_url,
            free_member_limit,
            free_restricted,
            ..
        } => {
            let link = format!("{app_url}/t/{slug}/billing");
            let items = free_limits(*free_member_limit, *free_restricted);
            let bullets: String = items.iter().map(|i| format!("- {i}\n")).collect();
            (
                format!("{team_name} moved to the Free plan"),
                format!(
                    "{team_name} is on the Free plan now. Nothing was deleted.\n\n\
                     {bullets}\n\
                     Go to billing:\n{link}\n"
                ),
                layout(
                    &format!(
                        "{} is on the Free plan now. Nothing was deleted.",
                        esc(team_name)
                    ),
                    &[
                        Block::List(items.iter().map(|i| esc(i)).collect()),
                        Block::Button("Go to billing", esc(&link)),
                    ],
                ),
            )
        }
    }
}

const MEMBER_ROLE: &str = "Members publish collections, share them and make publish tokens.";
const INVITE_FOOTER: &str = "The invite expires in 7 days. Not expecting it? Ignore this email.";
const FREE_KEEPS_SERVING: &str =
    "Restricted collections keep serving the people already on their lists.";
const FREE_OR_PUBLIC: &str =
    "Or make those collections public or off, and everything works on Free.";

/// What changes on Free, from the limits in force. With restricted
/// collections allowed there is nothing to say about them.
fn free_limits(member_limit: u32, restricted_allowed: bool) -> Vec<String> {
    if restricted_allowed {
        return vec![format!(
            "Free allows up to {member_limit} members. Upgrade to add more."
        )];
    }
    vec![
        FREE_KEEPS_SERVING.to_owned(),
        format!(
            "Until you upgrade, you can\u{2019}t publish to them, add people to them, \
             or add members past {member_limit}."
        ),
        FREE_OR_PUBLIC.to_owned(),
    ]
}

const FONT: &str = "-apple-system,BlinkMacSystemFont,'Segoe UI',Helvetica,Arial,sans-serif";
const MONO: &str = "ui-monospace,SFMono-Regular,Menlo,Consolas,monospace";

/// One row of an email's body. Strings are HTML the caller has escaped.
enum Block {
    Text(String),
    Muted(String),
    Code(String),
    Callout(String),
    List(Vec<String>),
    /// Label and escaped URL.
    Button(&'static str, String),
    Fine(String),
}

/// The light layout: mark, one heading, body, at most one button, fine print.
/// Tables and inline styles only, so mail clients render it.
fn layout(heading: &str, blocks: &[Block]) -> String {
    let mut rows = String::new();
    rows.push_str(&row(&format!(
        "<table role=\"presentation\" cellpadding=\"0\" cellspacing=\"0\" border=\"0\"><tr>\
         <td width=\"22\" height=\"22\" align=\"center\" bgcolor=\"#171717\" style=\"width:22px;\
         height:22px;border-radius:5px;background:#171717;color:#ffffff;font-family:{MONO};\
         font-weight:500;font-size:10px;line-height:22px;\">&gt;_</td>\
         <td style=\"padding-left:8px;font-weight:600;font-size:14px;color:#171717;\">\
         curl-runner</td></tr></table>"
    )));
    rows.push_str(&row(&format!(
        "<h1 style=\"margin:0;font-size:14px;line-height:22px;font-weight:400;color:#171717;\">\
         {heading}</h1>"
    )));
    for block in blocks {
        rows.push_str(&match block {
            Block::Text(t) => row(&format!("<p style=\"margin:0;\">{t}</p>")),
            Block::Muted(t) => row(&format!("<p style=\"margin:0;color:#444444;\">{t}</p>")),
            Block::Code(c) => row(&format!(
                "<div style=\"font-family:{MONO};font-size:28px;line-height:34px;\
                 letter-spacing:0.2em;font-weight:500;padding:14px 0;text-align:center;\
                 border:1px solid #ebebeb;border-radius:8px;background:#fafafa;\
                 color:#171717;\">{c}</div>"
            )),
            Block::Callout(t) => row(&format!(
                "<div style=\"padding:10px 12px;border:1px solid #ffe2a8;background:#fff6e5;\
                 border-radius:6px;font-size:13px;line-height:20px;color:#171717;\">{t}</div>"
            )),
            Block::List(items) => {
                let items: String = items
                    .iter()
                    .map(|i| format!("<li style=\"margin:0;\">{i}</li>"))
                    .collect();
                row(&format!(
                    "<ul style=\"margin:0;padding-left:18px;color:#444444;\">{items}</ul>"
                ))
            }
            Block::Button(label, url) => row(&format!(
                "<table role=\"presentation\" cellpadding=\"0\" cellspacing=\"0\" border=\"0\">\
                 <tr><td bgcolor=\"#171717\" style=\"border-radius:6px;background:#171717;\">\
                 <a href=\"{url}\" style=\"display:inline-block;padding:9px 18px;\
                 font-weight:500;font-size:14px;line-height:22px;color:#ffffff;\
                 text-decoration:none;\">{label}</a></td></tr></table>"
            )),
            Block::Fine(t) => row(&format!(
                "<p style=\"margin:0;font-size:12px;line-height:18px;color:#666666;\">{t}</p>"
            )),
        });
    }
    format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\">\
         <meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\
         <title>curl-runner</title></head>\
         <body style=\"margin:0;padding:0;background:#fafafa;\">\
         <table role=\"presentation\" width=\"100%\" cellpadding=\"0\" cellspacing=\"0\" border=\"0\" \
         bgcolor=\"#fafafa\" style=\"background:#fafafa;\"><tr>\
         <td align=\"center\" style=\"padding:32px 24px;\">\
         <table role=\"presentation\" width=\"440\" cellpadding=\"0\" cellspacing=\"0\" border=\"0\" \
         bgcolor=\"#ffffff\" style=\"width:100%;max-width:440px;background:#ffffff;\
         border:1px solid #ebebeb;border-radius:10px;\"><tr>\
         <td style=\"padding:28px 28px 12px;font-family:{FONT};font-size:14px;\
         line-height:22px;color:#171717;\">\
         <table role=\"presentation\" width=\"100%\" cellpadding=\"0\" cellspacing=\"0\" border=\"0\">\
         {rows}</table></td></tr></table></td></tr></table></body></html>"
    )
}

/// A body row with 16px of space below it.
fn row(inner: &str) -> String {
    format!("<tr><td style=\"padding:0 0 16px;\">{inner}</td></tr>")
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
            link: Some("https://cloud.example.com/auth/magic?token=abc".into()),
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
    fn device_sign_in_template_has_code_and_no_link() {
        let (subject, text, html) = render(&Mail::SignInCode {
            to: "ada@example.com".into(),
            code: "123456".into(),
            link: None,
        });
        assert_eq!(subject, "Your curl-runner sign-in code: 123456");
        assert!(text.contains("123456") && html.contains("123456"));
        assert!(text.contains("Enter it in your terminal."));
        for s in [&subject, &text, &html] {
            assert!(!s.contains("http"), "{s}");
            assert!(!s.contains("<a "), "{s}");
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
    fn subject_has_no_line_breaks() {
        let (subject, _, _) = render(&Mail::CustomerInvite {
            to: "b@example.com".into(),
            team_name: "Acme\r\nBcc: x@example.com".into(),
            collection_name: "pay\nments".into(),
            link: "https://x.example.com/c".into(),
        });
        assert_eq!(
            subject,
            "Acme  Bcc: x@example.com shared pay ments with you"
        );
        assert!(!subject.contains(['\r', '\n']));
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
        assert!(!html.contains("<b><b>"), "user text must not add tags");
    }

    fn every_template() -> Vec<Mail> {
        let deadline = "2026-11-19T10:00:00Z".parse().unwrap();
        vec![
            sign_in(),
            Mail::SignInCode {
                to: "ada@example.com".into(),
                code: "123456".into(),
                link: None,
            },
            Mail::MemberInvite {
                to: "b@example.com".into(),
                team_name: "Acme".into(),
                inviter: "Ada".into(),
                link: "https://x.example.com/i".into(),
            },
            Mail::CustomerInvite {
                to: "b@example.com".into(),
                team_name: "Acme".into(),
                collection_name: "payments-api".into(),
                link: "https://x.example.com/c".into(),
            },
            Mail::PaymentFailed {
                to: "ada@example.com".into(),
                team_name: "Acme".into(),
                slug: "acme".into(),
                deadline,
                app_url: "https://app.example.com".into(),
            },
            Mail::PlanChanged {
                to: "ada@example.com".into(),
                team_name: "Acme".into(),
                slug: "acme".into(),
                app_url: "https://app.example.com".into(),
                free_member_limit: 3,
                free_restricted: false,
            },
            Mail::TestEmail {
                to: "ada@example.com".into(),
            },
        ]
    }

    #[test]
    fn plan_changed_template_has_no_em_dash() {
        for mail in every_template() {
            let (subject, text, html) = render(&mail);
            for s in [&subject, &text, &html] {
                assert!(!s.contains('\u{2014}'), "{mail:?}: {s}");
            }
        }
    }

    #[test]
    fn every_template_uses_the_light_layout() {
        for mail in every_template() {
            let (_, text, html) = render(&mail);
            assert!(html.contains("curl-runner</td>"), "{mail:?}");
            assert!(html.contains("&gt;_"), "{mail:?}");
            assert_eq!(html.matches("<h1 ").count(), 1, "{mail:?}");
            assert!(html.matches("<a ").count() <= 1, "{mail:?}");
            assert!(html.contains("max-width:440px"), "{mail:?}");
            assert!(!text.is_empty());
        }
    }

    #[test]
    fn payment_failed_copy_and_link() {
        let (subject, text, html) = render(&every_template()[4]);
        assert_eq!(subject, "We couldn't take Acme's payment");
        assert!(text.contains(
            "We couldn't take the last payment for Acme. Update your card by Nov 19 \
             to keep the Team plan."
        ));
        for s in [&text, &html] {
            assert!(s.contains("https://app.example.com/t/acme/billing"), "{s}");
        }
        assert!(html.contains("<b>Nov 19</b>"));
    }

    #[test]
    fn plan_changed_copy_and_link() {
        let (subject, text, html) = render(&every_template()[5]);
        assert_eq!(subject, "Acme moved to the Free plan");
        assert!(text.contains("Nothing was deleted."));
        assert!(text.contains("Restricted collections keep serving the people already"));
        assert!(text.contains("add members past 3."));
        for s in [&text, &html] {
            assert!(s.contains("https://app.example.com/t/acme/billing"), "{s}");
        }
    }

    fn plan_changed(limit: u32, restricted: bool) -> (String, String, String) {
        render(&Mail::PlanChanged {
            to: "ada@example.com".into(),
            team_name: "Acme".into(),
            slug: "acme".into(),
            app_url: "https://app.example.com".into(),
            free_member_limit: limit,
            free_restricted: restricted,
        })
    }

    #[test]
    fn plan_changed_states_the_configured_limit() {
        let (_, text, html) = plan_changed(7, false);
        for s in [&text, &html] {
            assert!(s.contains("add members past 7."), "{s}");
            assert!(!s.contains("past 3"), "{s}");
            assert!(s.contains("publish to them"), "{s}");
            assert!(!s.contains('\u{2014}'), "{s}");
        }
    }

    #[test]
    fn plan_changed_drops_the_restricted_clause_when_allowed() {
        let (_, text, html) = plan_changed(5, true);
        for s in [&text, &html] {
            assert!(s.contains("Free allows up to 5 members."), "{s}");
            assert!(!s.contains("publish to them"), "{s}");
            assert!(!s.contains("Restricted collections"), "{s}");
            assert!(!s.contains('\u{2014}'), "{s}");
        }
    }
}
