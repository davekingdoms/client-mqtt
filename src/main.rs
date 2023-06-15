use csv::WriterBuilder;
use drogue_ttn::v3::{Message, Payload, DataRate};
use futures::{executor::block_on, stream::StreamExt};
use paho_mqtt::{self as mqtt};
use std::{process, time::Duration, fs::{OpenOptions}};
use std::path::Path;

// TODO: Replace with your own TTN identifiers
const HOST: &str = "eu1.cloud.thethings.network:1883";
const TOPICS: [&str; 2] = [
    "v3/prova-07-02@ttn/devices/eui-stm32wl-rust/join",
    "v3/prova-07-02@ttn/devices/eui-stm32wl-rust/up",
];
const QOS: [i32; 2] = [1, 1];
const USERNAME: &str = "prova-07-02@ttn";
const PASSWORD: &str = "NNSXS.REZ3CZK4MCLMNC56VDWBFURJVNA6CGJTS5SRBXQ.FO4OHC6SMFJZL64ZBMATWCKFTLFL22MWLE3ZZUCOKZEOZTL6JDEA";

fn main() {
    // Initialize the logger from the environment
    env_logger::init();



    // Create the client. Use a Client ID for a persistent session.
    // A real system should try harder to use a unique ID.

    // Create the client connection
    let mut cli = mqtt::AsyncClient::new(HOST).unwrap_or_else(|e| {
        println!("Error creating the client: {:?}", e);
        process::exit(1);
    });

    if let Err(err) = block_on(async {
        // Get message stream before connecting.
        let mut strm = cli.get_stream(244);

        // Define the set of options for the connection
      let lwt = mqtt::Message::new("test", "Async subscriber lost connection", mqtt::QOS_1);

        // Create the connect options, explicitly requesting MQTT v3.x
        let opts = mqtt::ConnectOptionsBuilder::new()
        .user_name(USERNAME)
        .password(PASSWORD)
        .keep_alive_interval(Duration::from_secs(30))
        .clean_session(false)
        .will_message(lwt)
        .finalize();

        // Make the connection to the broker
        println!("Connecting to the MQTT server...");
        cli.connect(opts).await?;

        println!("Subscribing to topics: {:?}", TOPICS);
        cli.subscribe_many(&TOPICS, &QOS).await?;

       let path = "/home/alleregni/csvexample.csv";
  
      let mut options = OpenOptions::new();
      options.write(true).create(true).append(true);
  
      let file_exists = Path::new(path).exists();
  
      let mut csv_writer = match options.open(path) {
          Ok(file) => {
              if file_exists {
                  WriterBuilder::new()
                      .has_headers(false)
                      .from_writer(file)
              } else {
                  let mut csv_writer = WriterBuilder::new()
                      .has_headers(true)
                      .from_writer(file);
  
                  let header = ["GatewayId", "UTC Time","Latitude","Longitude","Altitude","RSSI","SNR","Spreading Factor"];
                  csv_writer.write_record(&header).expect("Errore durante la scrittura dell'intestazione");
                  csv_writer.flush().expect("Errore durante il flushing del writer");
  
                  csv_writer
              }
              

              
          }
          Err(_) => panic!("Impossibile aprire il file"),
      };
        // Just loop on incoming messages.
        println!("Waiting for messages...");

        // Note that we're not providing a way to cleanly shut down and
        // disconnect. Therefore, when you kill this app (with a ^C or
        // whatever) the server will get an unexpected drop and then
        // should emit the LWT message.
        while let Some(msg_opt) = strm.next().await {
            if let Some(raw) = msg_opt {
                if let Ok(payload) =
                serde_json::from_slice::<Message>(raw.payload()).map(|message| message.payload)
            {
                match payload {
                    Payload::JoinAccept(_) => println!("device joined"),


                    Payload::Uplink(uplink) => {

                        let buf = uplink.frame_payload;
                        println!("frame payload: {:?}",buf);

                        println!("{:?}", buf.len());

                        let latitude = Some(f64::from_le_bytes(buf[0..8].try_into().unwrap())).unwrap();
                        println!("latitude {:?}", latitude);
                        let longitude = Some(f64::from_le_bytes(buf[8..16].try_into().unwrap())).unwrap();
                        println!("Longitude {:?}", longitude);
                        let altitude = Some(f32::from_le_bytes(buf[16..20].try_into().unwrap())).unwrap();              
                        println!("altitude {:?}", altitude);
                       
                       let time = uplink.received_at;
                       let sf = match uplink.settings.data_rate {
                        Some(DataRate::Lora(data_rate_lora)) => data_rate_lora.spreading_factor,
                        _ => 0, // Valore di fallback nel caso in cui il campo sia None o non sia di tipo Lora
                    };
                    println!("Sf: {}",sf);

                        let mut i = 0;
                        while i < uplink.rx_metadata.len(){
                            let gateway_id =  uplink.rx_metadata[i].gateway_ids.get("gateway_id").unwrap();
                            if gateway_id == "dlos"{
                                let snr = uplink.rx_metadata[i].snr.unwrap();
                                let rssi = uplink.rx_metadata[i].rssi;
                                println!("SNR: {}, RSSI:{}",snr,rssi);
                                csv_writer.write_record(&[gateway_id.to_string(), time.to_string(), latitude.to_string(), longitude.to_string(), altitude.to_string(), rssi.to_string(), snr.to_string(),sf.to_string()]).expect("Errore durante la scrittura del record");
                                csv_writer.flush().expect("Errore durante il flushing del writer");
                                break;
                                 }

                            else{i +=1;}
                            }

                        }
                        
                    }
                 }
            }
            else {
                // A "None" means we were disconnected. Try to reconnect...
                println!("Lost connection. Attempting reconnect.");
                while let Err(err) = cli.reconnect().await {
                    println!("Error reconnecting: {}", err);
                    // For tokio use: tokio::time::delay_for()
                    async_std::task::sleep(Duration::from_millis(1000)).await;
                }
            }
        }
        
     
        // Explicit return type for the async block
        Ok::<(), mqtt::Error>(())
    }) {
        eprintln!("{}", err);
    }

   
}