/// Human release labels use Pitman's dozenal digits; Cargo keeps numeric SemVer.
pub fn dozenal(mut value: u64) -> String {
    let digits = ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9', '↊', '↋'];
    let mut result = Vec::new();
    loop {
        if let Some(digit) = digits.get((value % 12) as usize) {
            result.push(*digit);
        }
        value /= 12;
        if value == 0 {
            break;
        }
    }
    result.into_iter().rev().collect()
}
pub fn label(stable: u64, subdivision: u64) -> String {
    format!("v0.{}.{}", dozenal(stable), dozenal(subdivision))
}
