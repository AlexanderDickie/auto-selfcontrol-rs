use chumsky::prelude::*;
use chrono::NaiveTime;
use chrono::Weekday;
use super::{Day, Blocks, Paths};

/*
[paths]
selfcontrol = "..."
launch_agents = "..."

[blocks]

* = [
    (09:30 -> 13:50),
    (20:00 -> 08:30)
]

Mon = [(12:00 -> 15:00)]

[Sat, Sun] = [
    (11:00 -> 12:15),
    (14:30 -> 18:36)
]
*/
fn parse_config_paths() -> impl Parser<char, Paths, Error = Simple<char>> {
    let heading = just("paths")
        .delimited_by(
            just('[').padded(),
            just(']').padded()
    );

    let str = just('"')
        .ignore_then(none_of('"').repeated())
        .then_ignore(just('"'))
        .collect::<String>();

    let path_ident = |name: String| {
        just(name.clone())
            .padded()
            .ignore_then(just('=').padded())
            .then(str.clone())
            .map(|(_, s)| s)
    };

    heading
        .padded()
        .ignore_then(path_ident("selfcontrol".into()).padded())
        .then(path_ident("launch_agents".into()).padded())
}

fn parse_times() -> impl Parser<char, Vec<(NaiveTime, NaiveTime)>, Error = Simple<char>> {
    let digit = one_of("0123456789");
    // 09:30
    let time = one_of("012")
        .then(digit.clone())
        .then_ignore(just(':'))
        .then(one_of("012345"))
        .then(digit.clone())
        .map(|(((a,b),c),d)| format!("{}{}:{}{}", a,b,c,d))
        .map(|s| NaiveTime::parse_from_str(&s, "%H:%M").unwrap());

    let time_pair = time.clone()
        .padded()
        .then_ignore(just("->"))
        .then(time.padded())
        .delimited_by(
            just('('),
            just(')'),
        );

    time_pair
        .padded()
        .separated_by(just(','))
        .delimited_by(just('['), just(']'))
}


fn parse_config_blocks() -> impl Parser<char, Blocks, Error = Simple<char>> {
    let heading = just("blocks")
        .padded()
        .delimited_by(
            just('['),
            just(']'),
    );

    let day = choice((
        just("*").to(Day::Default),
        just("Mon").to(Day::Weekday(Weekday::Mon)),
        just("Tue").to(Day::Weekday(Weekday::Tue)),
        just("Wed").to(Day::Weekday(Weekday::Wed)),
        just("Thu").to(Day::Weekday(Weekday::Thu)),
        just("Fri").to(Day::Weekday(Weekday::Fri)),
        just("Sat").to(Day::Weekday(Weekday::Sat)),
        just("Sun").to(Day::Weekday(Weekday::Sun)),
    ));

    let key = day.clone().padded()
        .separated_by(just(','))
        .delimited_by(just('['), just(']'))
        .or(day.map(|day| vec![day]));


    heading.padded()
        .ignore_then(
            key.padded()
            .then_ignore(just('=').padded())
            .then(parse_times().padded())
            .repeated()
        )
}

pub fn parse_config() -> impl Parser<char, (Paths, Blocks), Error = Simple<char>> {
    parse_config_paths()
        .padded()
        .then(parse_config_blocks())
        .or(
            parse_config_blocks().padded()
            .then(parse_config_paths())
            .map(|(p, b)| (b, p))
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_config_paths() {
        let input = r#"
        [paths]
        selfcontrol = "../Users/username/Library/Application Support/SelfControl"
        launch_agents = "/Users/username/Library/LaunchAgents"
        "#;

        let (selfcontrol_path, launch_agents_path) = parse_config_paths().parse(input).unwrap();

        assert_eq!(selfcontrol_path, "../Users/username/Library/Application Support/SelfControl");
        assert_eq!(launch_agents_path, "/Users/username/Library/LaunchAgents");
    }
    #[test]
    fn test_missing_config_paths() {
        let input = r#"
        [paths]
        selfcontrol = "../Users/username/Library/Application Support/SelfControl"
        "#;

        let result = parse_config_paths().parse(input);

        assert!(result.is_err());
    }
    #[test]
    fn test_parse_times() {
        let input = r#"[
        (09:30 -> 13:50),
        (20:00 -> 08:30)
        ]"#;

        let times = parse_times().parse(input).unwrap();
        println!("{:?}", times);

        assert_eq!(times, vec![
            (NaiveTime::from_hms(9, 30, 0), NaiveTime::from_hms(13, 50, 0)),
            (NaiveTime::from_hms(20, 0, 0), NaiveTime::from_hms(8, 30, 0)),
        ]);
    }


    #[test]
    fn test_empty_config_blocks() {
        let input = r#"
        [blocks]
        "#;

        let blocks = parse_config_blocks().parse(input).unwrap();
        println!("{:?}", blocks);

        assert_eq!(blocks.len(), 0);
    }
    #[test]
    fn test_parse_config_blocks() {
        let input = r#"
        [blocks]
        * = [
            ( 09:30 -> 13:50),
            (20:00 -> 08:30 )
        ]

        Mon = [[12:00 -> 15:00]]

        [Sat, Sun] = [

            (11:00 -> 12:15),

            (14:30 -> 18:36)
        ]
        "#;

        let blocks = parse_config_blocks().parse(input).unwrap();
        print!("{:?}",blocks);

        for block in blocks {
            match block.0[0] {
                Day::Default => assert_eq!(block.1, vec![
                    (NaiveTime::from_hms(9, 30, 0), NaiveTime::from_hms(13, 50, 0)),
                    (NaiveTime::from_hms(20, 0, 0), NaiveTime::from_hms(8, 30, 0)),
                ]),
                Day::Weekday(Weekday::Mon) => assert_eq!(block.1, vec![
                    (NaiveTime::from_hms(12, 0, 0), NaiveTime::from_hms(15, 0, 0)),
                ]),
                Day::Weekday(Weekday::Sat) => assert_eq!(block.1, vec![
                    (NaiveTime::from_hms(11, 0, 0), NaiveTime::from_hms(12, 15, 0)),
                    (NaiveTime::from_hms(14, 30, 0), NaiveTime::from_hms(18, 36, 0)),
                ]),
                Day::Weekday(Weekday::Sun) => assert_eq!(block.1, vec![
                    (NaiveTime::from_hms(11, 0, 0), NaiveTime::from_hms(12, 15, 0)),
                    (NaiveTime::from_hms(14, 30, 0), NaiveTime::from_hms(18, 36, 0)),
                ]),
                _ => panic!("Unexpected day"),
            }
        }
    }

    #[test]
    fn test_parse_config() {
        let input = r#"
        [paths]
        selfcontrol = "../Users/username/Library/Application_Support/SelfControl"
        launch_agents = "/Users/username/Library/LaunchAgents"

        [blocks]
        * = [
            ( 09:30 -> 13:50),
            (20:00 -> 08:30 )
        ]

        Mon = [(12:00 -> 15:00)]

        [Sat, Sun] = [

            (11:00 -> 12:15),

            (14:30 -> 18:36)
        ]
        "#;

        let (paths, blocks) = parse_config().parse(input).unwrap();

        assert_eq!(paths.0, "../Users/username/Library/Application_Support/SelfControl");
        assert_eq!(paths.1, "/Users/username/Library/LaunchAgents");
        assert_eq!(blocks.len(), 3);
    }
}
