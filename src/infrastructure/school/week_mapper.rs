use chrono::{Weekday, WeekdaySet};

/// 将学校API的中文星期字符串转换为 Domain 层的集合
fn parse_school_week(week_str: &str) -> WeekdaySet {
    let mut set = WeekdaySet::default();
    for part in week_str.split(',') {
        match part.trim() {
            "星期一" => { set.insert(Weekday::Mon); }
            "星期二" => { set.insert(Weekday::Tue); }
            "星期三" => { set.insert(Weekday::Wed); }
            "星期四" => { set.insert(Weekday::Thu); }
            "星期五" => { set.insert(Weekday::Fri); }
            "星期六" => { set.insert(Weekday::Sat); }
            "星期日" => { set.insert(Weekday::Sun); }
            _ => {}
        }
    }
    set
}