use std::process::{Command, Stdio};
use std::io::{BufRead, BufReader, Error, ErrorKind};
use std::time::{SystemTime};
use cloudevents::{EventBuilder, EventBuilderV10, Event};
use serde_json::json;
use url::Url;

fn temp_to_cloudevent(temp: f64) -> Event {
  let epoch = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).expect("could not get time"); 
  let epoch_float = epoch.as_secs_f64(); 
  let payload = json!({"loc": "bedroom", "dt":  epoch_float, "temp": temp});
  let event = EventBuilderV10::new()
    .source(Url::parse("http://lytro.wellorder.net/iot/mcp9808").unwrap())
    .subject("bedroom")
    .ty("com.wellorder.iot.indoorenv")
    .data("application/json", payload)
    .build().unwrap();
  event
}

#[async_std::main]
async fn main() -> Result<(), Error> {
    println!("Connecting to NATS");
    let ncres = nats::connect("nats://nats.wellorder.net:4222");
    let nc = match ncres {
        Ok(conn) => conn,
        Err(e) => {
            println!("Could not connect, bailing");
            std::process::exit(1);
        }
    };
    let nc = nats::connect("nats.")?;
    let stdout = Command::new("python")
        .arg("/home/pi/read_temp.py")
        .stdout(Stdio::piped())
        .spawn()?
        .stdout
        .ok_or_else(|| Error::new(ErrorKind::Other,"Could not capture standard output."))?;

    let reader = BufReader::new(stdout);
    reader
        .lines()
        .filter_map(|line| line.ok())
        .map(|x| temp_to_cloudevent(x.parse::<f64>().unwrap()))
        .map(|x| serde_json::to_string(&x).unwrap())
        .for_each(|event| {
            nc.publish("iot.indoorenv", event).expect("publish failed");
            println!("published");
        });
     Ok(())
}
