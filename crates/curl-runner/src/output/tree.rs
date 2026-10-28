//! Tree-style output (`core/format/tree-renderer.ts`).

use crate::colors::{Color, colorize};

#[derive(Debug, Clone, Default)]
pub struct TreeNode {
    pub label: String,
    pub value: Option<String>,
    pub children: Vec<TreeNode>,
    pub color: Option<Color>,
}

impl TreeNode {
    pub fn leaf(label: impl Into<String>, value: impl Into<String>, color: Option<Color>) -> Self {
        TreeNode {
            label: label.into(),
            value: Some(value.into()),
            children: Vec::new(),
            color,
        }
    }

    pub fn section(label: impl Into<String>, children: Vec<TreeNode>) -> Self {
        TreeNode {
            label: label.into(),
            value: None,
            children,
            color: None,
        }
    }
}

/// Render `nodes` into lines (TS `TreeRenderer.render`, default prefix 3 spaces).
pub fn render(nodes: &[TreeNode], color: bool) -> Vec<String> {
    let mut lines = Vec::new();
    render_into(&mut lines, nodes, "   ", color);
    lines
}

fn render_into(lines: &mut Vec<String>, nodes: &[TreeNode], base: &str, color: bool) {
    for (i, node) in nodes.iter().enumerate() {
        let last = i + 1 == nodes.len();
        let prefix = format!("{base}{}", if last { "└─" } else { "├─" });
        let cont = format!("{base}{}", if last { "   " } else { "│  " });
        let value = node.value.as_deref().filter(|v| !v.is_empty());
        match (node.label.is_empty(), value) {
            (false, Some(v)) => {
                let shown = match node.color {
                    Some(c) if color => colorize(v, c),
                    _ => v.to_string(),
                };
                if shown.contains('\n') {
                    lines.push(format!("{prefix} {}:", node.label));
                    lines.extend(shown.split('\n').map(|l| format!("{cont}{l}")));
                } else {
                    lines.push(format!("{prefix} {}: {shown}", node.label));
                }
            }
            (false, None) => lines.push(format!("{prefix} {}:", node.label)),
            (true, Some(v)) => lines.push(format!("{cont}{v}")),
            (true, None) => {}
        }
        if !node.children.is_empty() {
            render_into(lines, &node.children, &cont, color);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_branches_sections_and_multiline() {
        let nodes = vec![
            TreeNode::leaf("URL", "http://x", Some(Color::Blue)),
            TreeNode::section(
                "Headers",
                vec![
                    TreeNode::leaf("a", "1", None),
                    TreeNode::leaf("b", "2", None),
                ],
            ),
            TreeNode::leaf("Response Body", "{\n  \"a\": 1\n}", None),
        ];
        assert_eq!(
            render(&nodes, false),
            vec![
                "   ├─ URL: http://x",
                "   ├─ Headers:",
                "   │  ├─ a: 1",
                "   │  └─ b: 2",
                "   └─ Response Body:",
                "      {",
                "        \"a\": 1",
                "      }",
            ]
        );
        assert_eq!(
            render(&nodes[..1], true),
            vec!["   └─ URL: \x1b[34mhttp://x\x1b[0m"]
        );
    }
}
