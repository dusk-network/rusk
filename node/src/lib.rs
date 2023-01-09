use dusk_consensus::user::provisioners::Provisioners;

pub enum Error {}

/// Reads config parameters
pub fn read_config() -> Result<(), Error> {
    Ok(())
}

///  Sets up a node and execute lifecycle loop.
pub fn bootstrap() -> Result<(), Error> {
    let empty_list = Provisioners::new();
    println!("Hello provisioners {:?}", empty_list);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
}
