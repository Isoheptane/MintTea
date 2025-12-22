use uuid::Uuid;

pub enum MonitorCommandParseResult {
    AddRule,
    ListRules,
    RemoveRule(Option<Result<Uuid, uuid::Error>>),
    Help,
    NotMatch
}

pub fn parse_monitor_command(text: &str) -> MonitorCommandParseResult {
    let args: Vec<&str> = text.split_whitespace().collect();

    let Some(subcommand) = args.get(1) else {
        return MonitorCommandParseResult::AddRule;
    };

    match *subcommand {
        "help" => return MonitorCommandParseResult::Help,
        "list" => return MonitorCommandParseResult::ListRules,
        "remove" => return MonitorCommandParseResult::RemoveRule(
            args.get(2).map(|s| Uuid::parse_str(s))
        ),
        _ => return MonitorCommandParseResult::NotMatch
    }
}