use chrono::Local;

pub fn format_now_date() -> String {
    let fmt = "%Y-%m-%d %H:%M:%S";
    Local::now().format(fmt).to_string()
}

#[cfg(test)]
pub mod time_util_tests {
    use crate::node::dal::utils::format_now_date;

    #[test]
    fn test_format_now_date() {
        let res = format_now_date();
        println!("{:?}", res);
    }
}
