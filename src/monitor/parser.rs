use uuid::Uuid;

pub enum MonitorCommandParseResult {
    AddRule,
    AddRuleByForward,
    AddRuleByReply,
    ListRules,
    RemoveRule(Option<Result<Uuid, uuid::Error>>),
    RemoveAllRule,
    Help,
    NotMatch
}

pub fn parse_monitor_command(text: &str) -> MonitorCommandParseResult {
    let args: Vec<&str> = text.split_whitespace().collect();

    let Some(subcommand) = args.get(1) else {
        return MonitorCommandParseResult::Help;
    };

    match *subcommand {
        "add" | "a" => {
            if let Some(add_type) = args.get(2) {
                match *add_type {
                    "f" | "forward" => return MonitorCommandParseResult::AddRuleByForward,
                    "r" | "reply" => return MonitorCommandParseResult::AddRuleByReply,
                    _ => return MonitorCommandParseResult::NotMatch,
                }
            } else {
                return MonitorCommandParseResult::AddRule;
            }
        }
        "help" => return MonitorCommandParseResult::Help,
        "rules" | "ls" | "list" => return MonitorCommandParseResult::ListRules,
        "remove" | "rm" => return MonitorCommandParseResult::RemoveRule(
            args.get(2).map(|s| Uuid::parse_str(s))
        ),
        "removeall" | "rmall" => return MonitorCommandParseResult::RemoveAllRule,
        _ => return MonitorCommandParseResult::NotMatch
    }
}