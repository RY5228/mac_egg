use libertyparse::{Liberty, PinDirection};
use std::fs;
use std::path::Path;
pub fn read_liberty<P: AsRef<Path>>(path: P) -> Result<Liberty, String> {
    let content =
        fs::read_to_string(path.as_ref()).map_err(|e| format!("Error reading file: {}", e))?;
    Liberty::parse_str(&content)
}

pub fn get_direction_of_pins(
    liberty: &Liberty,
) -> Result<Vec<(String, Vec<(String, &PinDirection)>)>, String> {
    Ok(liberty
        .libs
        .first()
        .ok_or("No lib found!".to_string())?
        .1
        .cells
        .iter()
        .map(|(n, c)| {
            (
                n.to_string(),
                c.pins
                    .iter()
                    .map(|(n, p)| (n.to_string(), &p.direction))
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_liberty() {
        let liberty = read_liberty("test/asap7sc6t_SELECT_LVT_TT_nldm.lib").unwrap();
        assert_eq!(liberty.libs.len(), 1);
        let lib = liberty.libs.first().unwrap();
        println!("{}:", lib.0);
        for cell in lib.1.cells.iter() {
            println!("  {}:", cell.0);
            for pin in cell.1.pins.iter() {
                println!("    {}: {:?}", pin.0, pin.1.direction);
            }
        }
        let pins_direction = get_direction_of_pins(&liberty).unwrap();
        println!("{:?}", pins_direction);
    }
}
