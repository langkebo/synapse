use serde::{Deserialize, Serialize};
use std::borrow::Cow;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PushAction {
    Notify,
    DontNotify,
    Coalesce,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushRule {
    #[serde(borrow)]
    pub rule_id: Cow<'static, str>,
    pub default: bool,
    pub enabled: bool,
    pub actions: Vec<ActionKind>,
    pub conditions: Vec<Condition>,
    pub priority_class: PriorityClass,
}

impl PushRule {
    pub fn static_rule(
        rule_id: &'static str,
        default: bool,
        enabled: bool,
        actions: Vec<ActionKind>,
        conditions: Vec<Condition>,
        priority_class: PriorityClass,
    ) -> Self {
        Self {
            rule_id: Cow::Borrowed(rule_id),
            default,
            enabled,
            actions,
            conditions,
            priority_class,
        }
    }

    pub fn dynamic_rule(
        rule_id: String,
        default: bool,
        enabled: bool,
        actions: Vec<ActionKind>,
        conditions: Vec<Condition>,
        priority_class: PriorityClass,
    ) -> Self {
        Self {
            rule_id: Cow::Owned(rule_id),
            default,
            enabled,
            actions,
            conditions,
            priority_class,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionKind {
    #[serde(rename = "notify")]
    Notify,
    #[serde(rename = "dont_notify")]
    DontNotify,
    #[serde(rename = "coalesce")]
    Coalesce,
    #[serde(rename = "set_tweak")]
    SetTweak { set_tweak: String, value: serde_json::Value },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PriorityClass {
    Override = 0,
    ContentSpecific = 1,
    RoomSpecific = 2,
    SenderSpecific = 3,
    Underride = 4,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum Condition {
    #[serde(rename = "event_match")]
    EventMatch {
        key: String,
        pattern: String,
    },
    #[serde(rename = "contains_display_name")]
    ContainsDisplayName,
    #[serde(rename = "room_member_count")]
    RoomMemberCount {
        #[serde(default)]
        is: Option<String>,
    },
    #[serde(rename = "sender_notification_permission")]
    SenderNotificationPermission {
        key: String,
    },
}

#[derive(Debug, Clone)]
pub struct EventContext {
    pub event_type: String,
    pub sender: String,
    pub room_id: String,
    pub content: serde_json::Value,
    pub state_key: Option<String>,
    pub display_name: Option<String>,
    pub room_member_count: usize,
    pub user_power_level: i64,
}

pub struct PushRuleEvaluator {
    override_rules: Vec<PushRule>,
    content_rules: Vec<PushRule>,
    room_rules: Vec<PushRule>,
    sender_rules: Vec<PushRule>,
    underride_rules: Vec<PushRule>,
}

impl PushRuleEvaluator {
    pub fn new(rules: Vec<PushRule>) -> Self {
        let capacity = rules.len();
        let mut override_rules = Vec::with_capacity(capacity);
        let mut content_rules = Vec::with_capacity(capacity);
        let mut room_rules = Vec::with_capacity(capacity);
        let mut sender_rules = Vec::with_capacity(capacity);
        let mut underride_rules = Vec::with_capacity(capacity);

        for rule in rules {
            if !rule.enabled {
                continue;
            }
            match rule.priority_class {
                PriorityClass::Override => override_rules.push(rule),
                PriorityClass::ContentSpecific => content_rules.push(rule),
                PriorityClass::RoomSpecific => room_rules.push(rule),
                PriorityClass::SenderSpecific => sender_rules.push(rule),
                PriorityClass::Underride => underride_rules.push(rule),
            }
        }

        Self {
            override_rules,
            content_rules,
            room_rules,
            sender_rules,
            underride_rules,
        }
    }

    pub fn evaluate(&self, ctx: &EventContext) -> Option<PushAction> {
        if let Some(action) = self.evaluate_rules(&self.override_rules, ctx) {
            return Some(action);
        }

        if let Some(action) = self.evaluate_rules(&self.content_rules, ctx) {
            return Some(action);
        }

        if let Some(action) = self.evaluate_rules(&self.room_rules, ctx) {
            return Some(action);
        }

        if let Some(action) = self.evaluate_rules(&self.sender_rules, ctx) {
            return Some(action);
        }

        self.evaluate_rules(&self.underride_rules, ctx)
    }

    fn evaluate_rules(&self, rules: &[PushRule], ctx: &EventContext) -> Option<PushAction> {
        for rule in rules {
            if self.matches_rule(rule, ctx) {
                return Some(self.action_from_rule(rule));
            }
        }
        None
    }

    fn matches_rule(&self, rule: &PushRule, ctx: &EventContext) -> bool {
        if rule.conditions.is_empty() {
            return true;
        }

        for condition in &rule.conditions {
            if !self.matches_condition(condition, ctx) {
                return false;
            }
        }
        true
    }

    fn matches_condition(&self, condition: &Condition, ctx: &EventContext) -> bool {
        match condition {
            Condition::EventMatch { key, pattern } => {
                self.match_event_key(key, pattern, ctx)
            }
            Condition::ContainsDisplayName => {
                if let Some(display_name) = &ctx.display_name {
                    if let Some(body) = ctx.content.get("body").and_then(|b| b.as_str()) {
                        return body.to_lowercase().contains(&display_name.to_lowercase());
                    }
                }
                false
            }
            Condition::RoomMemberCount { is } => {
                if let Some(condition_str) = is {
                    self.match_member_count(condition_str, ctx.room_member_count)
                } else {
                    true
                }
            }
            Condition::SenderNotificationPermission { key: _ } => {
                ctx.user_power_level >= 50
            }
        }
    }

    fn match_event_key(&self, key: &str, pattern: &str, ctx: &EventContext) -> bool {
        let value = match key {
            "type" => ctx.event_type.as_str(),
            "sender" => ctx.sender.as_str(),
            "room_id" => ctx.room_id.as_str(),
            "state_key" => ctx.state_key.as_deref().unwrap_or(""),
            _ => {
                if let Some(v) = self.get_nested_value(&ctx.content, key) {
                    return self.glob_match(v, pattern);
                }
                return false;
            }
        };
        self.glob_match(value, pattern)
    }

    fn get_nested_value<'a>(&self, json: &'a serde_json::Value, path: &str) -> Option<&'a str> {
        let parts: Vec<&str> = path.split('.').collect();
        let mut current = json;

        for part in &parts[..parts.len().saturating_sub(1)] {
            current = current.get(part)?;
        }

        current.get(parts.last()?)?.as_str()
    }

    fn glob_match(&self, value: &str, pattern: &str) -> bool {
        if !pattern.contains('*') {
            return value == pattern;
        }

        let parts: Vec<&str> = pattern.split('*').collect();
        
        if parts.len() == 1 {
            return value == parts[0];
        }

        if !value.starts_with(parts[0]) {
            return false;
        }

        if !pattern.ends_with('*') {
            if let Some(last) = parts.last() {
                if !value.ends_with(last) {
                    return false;
                }
            }
        }

        let mut search_start = parts[0].len();
        for part in &parts[1..parts.len().saturating_sub(1)] {
            if part.is_empty() {
                continue;
            }
            if let Some(pos) = value[search_start..].find(part) {
                search_start += pos + part.len();
            } else {
                return false;
            }
        }
        true
    }

    fn match_member_count(&self, condition: &str, count: usize) -> bool {
        let condition = condition.trim();
        
        if let Ok(exact) = condition.parse::<usize>() {
            return count == exact;
        }

        if condition.starts_with("==") {
            if let Ok(n) = condition[2..].trim().parse::<usize>() {
                return count == n;
            }
        } else if condition.starts_with(">=") {
            if let Ok(n) = condition[2..].trim().parse::<usize>() {
                return count >= n;
            }
        } else if condition.starts_with("<=") {
            if let Ok(n) = condition[2..].trim().parse::<usize>() {
                return count <= n;
            }
        } else if condition.starts_with('>') {
            if let Ok(n) = condition[1..].trim().parse::<usize>() {
                return count > n;
            }
        } else if condition.starts_with('<') {
            if let Ok(n) = condition[1..].trim().parse::<usize>() {
                return count < n;
            }
        }

        false
    }

    fn action_from_rule(&self, rule: &PushRule) -> PushAction {
        for action in &rule.actions {
            match action {
                ActionKind::Notify => return PushAction::Notify,
                ActionKind::DontNotify => return PushAction::DontNotify,
                ActionKind::Coalesce => return PushAction::Coalesce,
                ActionKind::SetTweak { .. } => {}
            }
        }
        PushAction::DontNotify
    }

    pub fn get_default_rules() -> Vec<PushRule> {
        vec![
            PushRule::static_rule(
                ".m.rule.master",
                true,
                false,
                vec![ActionKind::DontNotify],
                vec![],
                PriorityClass::Override,
            ),
            PushRule::static_rule(
                ".m.rule.suppress_notices",
                true,
                true,
                vec![ActionKind::DontNotify],
                vec![
                    Condition::EventMatch {
                        key: "type".to_string(),
                        pattern: "m.room.message".to_string(),
                    },
                    Condition::EventMatch {
                        key: "content.msgtype".to_string(),
                        pattern: "m.notice".to_string(),
                    },
                ],
                PriorityClass::Override,
            ),
            PushRule::static_rule(
                ".m.rule.contains_display_name",
                true,
                true,
                vec![ActionKind::Notify, ActionKind::SetTweak {
                    set_tweak: "highlight".to_string(),
                    value: serde_json::json!(true),
                }],
                vec![Condition::ContainsDisplayName],
                PriorityClass::ContentSpecific,
            ),
            PushRule::static_rule(
                ".m.rule.room_one_to_one",
                true,
                true,
                vec![ActionKind::Notify, ActionKind::SetTweak {
                    set_tweak: "sound".to_string(),
                    value: serde_json::json!("default"),
                }],
                vec![Condition::RoomMemberCount { is: Some("2".to_string()) }],
                PriorityClass::Underride,
            ),
            PushRule::static_rule(
                ".m.rule.message",
                true,
                true,
                vec![ActionKind::Notify],
                vec![Condition::EventMatch {
                    key: "type".to_string(),
                    pattern: "m.room.message".to_string(),
                }],
                PriorityClass::Underride,
            ),
            PushRule::static_rule(
                ".m.rule.encrypted_room_one_to_one",
                true,
                true,
                vec![ActionKind::Notify, ActionKind::SetTweak {
                    set_tweak: "sound".to_string(),
                    value: serde_json::json!("default"),
                }],
                vec![
                    Condition::RoomMemberCount { is: Some("2".to_string()) },
                    Condition::EventMatch {
                        key: "type".to_string(),
                        pattern: "m.room.encrypted".to_string(),
                    },
                ],
                PriorityClass::Underride,
            ),
            PushRule::static_rule(
                ".m.rule.encrypted",
                true,
                true,
                vec![ActionKind::Notify],
                vec![Condition::EventMatch {
                    key: "type".to_string(),
                    pattern: "m.room.encrypted".to_string(),
                }],
                PriorityClass::Underride,
            ),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn create_test_context() -> EventContext {
        EventContext {
            event_type: "m.room.message".to_string(),
            sender: "@alice:example.com".to_string(),
            room_id: "!room:example.com".to_string(),
            content: json!({
                "msgtype": "m.text",
                "body": "Hello Bob!"
            }),
            state_key: None,
            display_name: Some("Bob".to_string()),
            room_member_count: 3,
            user_power_level: 0,
        }
    }

    #[test]
    fn test_display_name_match() {
        let rules = PushRuleEvaluator::get_default_rules();
        let evaluator = PushRuleEvaluator::new(rules);
        let ctx = create_test_context();

        let result = evaluator.evaluate(&ctx);
        assert_eq!(result, Some(PushAction::Notify));
    }

    #[test]
    fn test_glob_match_exact() {
        let evaluator = PushRuleEvaluator::new(vec![]);
        assert!(evaluator.glob_match("m.room.message", "m.room.message"));
        assert!(!evaluator.glob_match("m.room.message", "m.room.encrypted"));
    }

    #[test]
    fn test_glob_match_wildcard() {
        let evaluator = PushRuleEvaluator::new(vec![]);
        assert!(evaluator.glob_match("m.room.message", "m.room.*"));
        assert!(evaluator.glob_match("@alice:example.com", "@*:example.com"));
        assert!(!evaluator.glob_match("@alice:other.com", "@*:example.com"));
    }

    #[test]
    fn test_member_count_condition() {
        let evaluator = PushRuleEvaluator::new(vec![]);
        
        assert!(evaluator.match_member_count("2", 2));
        assert!(!evaluator.match_member_count("2", 3));
        assert!(evaluator.match_member_count(">=2", 3));
        assert!(evaluator.match_member_count("<=5", 4));
        assert!(evaluator.match_member_count(">2", 3));
        assert!(evaluator.match_member_count("<5", 4));
    }

    #[test]
    fn test_early_exit_on_override() {
        let rules = vec![
            PushRule::dynamic_rule(
                "override_rule".to_string(),
                true,
                true,
                vec![ActionKind::DontNotify],
                vec![Condition::EventMatch {
                    key: "type".to_string(),
                    pattern: "m.room.message".to_string(),
                }],
                PriorityClass::Override,
            ),
            PushRule::dynamic_rule(
                "content_rule".to_string(),
                true,
                true,
                vec![ActionKind::Notify],
                vec![Condition::ContainsDisplayName],
                PriorityClass::ContentSpecific,
            ),
        ];

        let evaluator = PushRuleEvaluator::new(rules);
        let ctx = create_test_context();

        let result = evaluator.evaluate(&ctx);
        assert_eq!(result, Some(PushAction::DontNotify));
    }
}
