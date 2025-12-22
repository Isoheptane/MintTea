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
        return MonitorCommandParseResult::AddRule;
    };

    match *subcommand {
        "forward" => return MonitorCommandParseResult::AddRuleByForward,
        "reply" => return MonitorCommandParseResult::AddRuleByReply,
        "help" => return MonitorCommandParseResult::Help,
        "rules" | "ls" | "list" => return MonitorCommandParseResult::ListRules,
        "rm" | "remove" => return MonitorCommandParseResult::RemoveRule(
            args.get(2).map(|s| Uuid::parse_str(s))
        ),
        "rmall" | "removeall" => return MonitorCommandParseResult::RemoveAllRule,
        _ => return MonitorCommandParseResult::NotMatch
    }
}