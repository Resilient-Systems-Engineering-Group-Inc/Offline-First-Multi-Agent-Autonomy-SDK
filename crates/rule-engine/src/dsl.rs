//! Rule DSL parser.

use crate::{Rule, RuleCondition, Action};
use anyhow::Result;

/// Rule DSL parser.
pub struct RuleParser;

impl RuleParser {
    /// Parse rule from DSL string.
    pub fn parse_rule(dsl: &str) -> Result<Rule> {
        // Simple DSL parser
        // Format:
        // RULE rule_id "Rule Name"
        // WHEN field == "value"
        // THEN SET field = "value"
        // PRIORITY 100

        let mut rule = Rule::new("default", "Unnamed Rule");
        let mut condition_parts = Vec::new();
        let mut action_parts = Vec::new();
        let mut in_when = false;
        let mut in_then = false;

        for line in dsl.lines() {
            let line = line.trim();
            
            if line.starts_with("RULE") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    rule.id = parts[1].to_string();
                }
                if parts.len() >= 4 {
                    rule.name = parts[2..].join(" ").trim_matches('"').to_string();
                }
            } else if line.starts_with("WHEN") {
                in_when = true;
                in_then = false;
                let condition = line.trim_start_matches("WHEN").trim();
                condition_parts.push(condition.to_string());
            } else if line.starts_with("THEN") {
                in_when = false;
                in_then = true;
                let action = line.trim_start_matches("THEN").trim();
                action_parts.push(action.to_string());
            } else if line.starts_with("AND") {
                if in_when {
                    let condition = line.trim_start_matches("AND").trim();
                    condition_parts.push(condition.to_string());
                }
            } else if line.starts_with("PRIORITY") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    rule.priority = parts[1].parse().unwrap_or(50);
                }
            } else if line.starts_with("ENABLED") {
                rule.enabled = true;
            } else if line.starts_with("DISABLED") {
                rule.enabled = false;
            } else if in_when && !line.is_empty() {
                condition_parts.push(line.to_string());
            } else if in_then && !line.is_empty() {
                action_parts.push(line.to_string());
            }
        }

        // Parse conditions
        rule.condition = parse_conditions(&condition_parts)?;

        // Parse actions
        for action_str in &action_parts {
            if let Some(action) = parse_action(action_str)? {
                rule.actions.push(action);
            }
        }

        Ok(rule)
    }
}

fn parse_conditions(parts: &[String]) -> Result<RuleCondition> {
    if parts.is_empty() {
        return Ok(RuleCondition::Always);
    }

    if parts.len() == 1 {
        return parse_single_condition(&parts[0]);
    }

    // Multiple conditions - use AND
    let mut conditions = Vec::new();
    for part in parts {
        conditions.push(parse_single_condition(part)?);
    }

    Ok(RuleCondition::And { conditions })
}

fn parse_single_condition(condition: &str) -> Result<RuleCondition> {
    let condition = condition.trim();

    // Parse == (equals)
    if let Some(pos) = condition.find("==") {
        let field = condition[..pos].trim().to_string();
        let value_str = condition[pos + 2..].trim();
        let value = parse_value(value_str);
        return Ok(RuleCondition::Equals { field, value });
    }

    // Parse != (not equals)
    if let Some(pos) = condition.find("!=") {
        let field = condition[..pos].trim().to_string();
        let value_str = condition[pos + 2..].trim();
        let value = parse_value(value_str);
        return Ok(RuleCondition::NotEquals { field, value });
    }

    // Parse > (greater than)
    if let Some(pos) = condition.find('>') {
        let field = condition[..pos].trim().to_string();
        let value_str = condition[pos + 1..].trim();
        let value = parse_value(value_str);
        return Ok(RuleCondition::GreaterThan { field, value });
    }

    // Parse < (less than)
    if let Some(pos) = condition.find('<') {
        let field = condition[..pos].trim().to_string();
        let value_str = condition[pos + 1..].trim();
        let value = parse_value(value_str);
        return Ok(RuleCondition::LessThan { field, value });
    }

    // Parse contains
    if let Some(pos) = condition.find("contains") {
        let field = condition[..pos].trim().to_string();
        let value_str = condition[pos + 8..].trim().trim_matches('(').trim_matches(')');
        let value = parse_value(value_str);
        return Ok(RuleCondition::Contains { field, value });
    }

    // Default: try to parse as exists
    Ok(RuleCondition::Exists { field: condition.to_string() })
}

fn parse_action(action_str: &str) -> Result<Option<Action>> {
    let action_str = action_str.trim();

    // Parse SET field = value
    if action_str.starts_with("SET") {
        let rest = action_str.trim_start_matches("SET").trim();
        if let Some(pos) = rest.find('=') {
            let field = rest[..pos].trim().to_string();
            let value_str = rest[pos + 1..].trim();
            let value = parse_value(value_str);
            return Ok(Some(Action::Set { field, value }));
        }
    }

    // Parse DELETE field
    if action_str.starts_with("DELETE") {
        let field = action_str.trim_start_matches("DELETE").trim().to_string();
        return Ok(Some(Action::Delete { field }));
    }

    // Parse INCREMENT field BY delta
    if action_str.starts_with("INCREMENT") {
        let rest = action_str.trim_start_matches("INCREMENT").trim();
        if let Some(pos) = rest.find("BY") {
            let field = rest[..pos].trim().to_string();
            let delta_str = rest[pos + 2..].trim();
            let delta = delta_str.parse().unwrap_or(1);
            return Ok(Some(Action::Increment { field, delta }));
        }
    }

    // Parse LOG "message"
    if action_str.starts_with("LOG") {
        let message = action_str.trim_start_matches("LOG").trim().trim_matches('"').to_string();
        return Ok(Some(Action::Log {
            level: "info".to_string(),
            message,
        }));
    }

    Ok(None)
}

fn parse_value(value_str: &str) -> serde_json::Value {
    let value_str = value_str.trim();

    // String value (quoted)
    if value_str.starts_with('"') && value_str.ends_with('"') {
        return serde_json::json!(value_str[1..value_str.len() - 1].to_string());
    }

    // Boolean
    if value_str == "true" {
        return serde_json::json!(true);
    }
    if value_str == "false" {
        return serde_json::json!(false);
    }

    // Null
    if value_str == "null" {
        return serde_json::json!(null);
    }

    // Number
    if let Ok(num) = value_str.parse::<i64>() {
        return serde_json::json!(num);
    }
    if let Ok(num) = value_str.parse::<f64>() {
        return serde_json::json!(num);
    }

    // Default: treat as string
    serde_json::json!(value_str.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_rule() {
        let dsl = r#"
RULE task-approval "Task Approval Rule"
WHEN priority == "high"
AND amount > 1000
THEN SET status = "pending_review"
SET requires_approval = true
LOG "High value task requires review"
PRIORITY 100
"#;

        let rule = RuleParser::parse_rule(dsl).unwrap();
        
        assert_eq!(rule.id, "task-approval");
        assert_eq!(rule.name, "Task Approval Rule");
        assert_eq!(rule.priority, 100);
        assert!(rule.enabled);
        assert_eq!(rule.actions.len(), 3);
    }

    #[test]
    fn test_parse_value() {
        assert_eq!(parse_value("\"hello\""), serde_json::json!("hello"));
        assert_eq!(parse_value("true"), serde_json::json!(true));
        assert_eq!(parse_value("false"), serde_json::json!(false));
        assert_eq!(parse_value("42"), serde_json::json!(42));
        assert_eq!(parse_value("3.14"), serde_json::json!(3.14));
        assert_eq!(parse_value("null"), serde_json::json!(null));
    }
}
