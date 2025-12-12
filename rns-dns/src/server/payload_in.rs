use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use reticulum::hash::AddressHash;
use x25519_dalek::PublicKey;

use nom::{
    IResult, Parser,
    bytes::complete::{tag, take_until},
    character::complete::one_of,
    sequence::{terminated, tuple},
};

pub enum RnsUri<'a> {
    Node(NodeUri<'a>), // N - generic node
    Destination,       // D - destination
}

#[derive(Debug)]
pub struct NodeUri<'a> {
    pub r#type: char,
    pub version: &'a str,
    pub category: &'a str,
    pub domain: &'a str,
    pub id: &'a str,
}

#[derive(Debug)]
pub struct RnsUria<'a> {
    pub r#type: char,
    pub version: &'a str,
    pub category: &'a str,
    pub domain: &'a str,
    pub id: &'a str,
}

fn segment(input: &str) -> IResult<&str, &str> {
    terminated(take_until("/"), tag("/")).parse(input)
}

pub fn parse_url(input: &str) -> IResult<&str, RnsUri> {
    let (input, _) = tag("rns://").parse(input)?;
    //rns://N/1/q5lkm7dw_5dPNm4nTAq3csvpdmk9smIXAKWohcRMnBQ/(/7c9fa136d4413fa6173637e883b6998d/ Udp;0.0.0.0:4243:127.0.0.1:4242),(/d86e8112f3c4c4442126f8e9f44f1686/ Tcp;0.0.0.0:53317)//

    // N - Generic Node, D - Destination, ...
    let (input, t) = one_of("DN").parse(input)?;
    let (input, _) = tag("/").parse(input)?;
    let (input, version) = segment(input)?;

    // log::info!("c{}", version);
    match t {
        'N' => {
            let (input, category) = segment(input)?;
        }
        'D' => {}
        _ => unreachable!("Unknown Type"),
    }

    // let (input, _) = tag("/").parse(input)?;
    // let (input, version) = segment(input)?;
    let (input, category) = segment(input)?;
    let (input, domain) = segment(input)?;
    let (input, id) = segment(input)?;

    Ok((
        input,
        RnsUri {
            r#type: t,
            version,
            category,
            domain,
            id,
        },
    ))
}

pub fn parse_rns(input: &str) -> IResult<&str, RnsUri> {
    let (input, _) = tag("rns://").parse(input)?;
    //rns://N/1/q5lkm7dw_5dPNm4nTAq3csvpdmk9smIXAKWohcRMnBQ/(/7c9fa136d4413fa6173637e883b6998d/ Udp;0.0.0.0:4243:127.0.0.1:4242),(/d86e8112f3c4c4442126f8e9f44f1686/ Tcp;0.0.0.0:53317)//

    let (input, t) = one_of("DNS").parse(input)?;
    let (input, _) = tag("/").parse(input)?;

    let (input, version) = segment(input)?;
    let (input, category) = segment(input)?;
    let (input, domain) = segment(input)?;
    let (input, id) = segment(input)?;

    Ok((
        input,
        RnsUri {
            r#type: t,
            version,
            category,
            domain,
            id,
        },
    ))
}

pub fn generate_node_url(
    version: &u16,
    address_hash: &Vec<AddressHash>,
    public_key: &PublicKey,
    interfaces: &Vec<crate::types::Connection>,
) -> String {
    let encoded_public_key = URL_SAFE_NO_PAD.encode(public_key);
    let interfaces = interfaces
        .iter()
        .map(|c| c.to_string())
        .collect::<Vec<String>>();
    let address_hash = address_hash
        .iter()
        .map(|c| c.to_string())
        .collect::<Vec<String>>();
    let zip = address_hash
        .iter()
        .zip(interfaces.iter())
        .map(|(hash, interface)| format!("({hash} {interface})"))
        .collect::<Vec<String>>()
        .join(",");

    format!("rns://N/{version}/{encoded_public_key}/{zip}//")
}

pub fn generate_destination_url(
    version: &u16,
    destination_name: &str,
    application_space: &str,
    address_hash: &Vec<AddressHash>,
) -> String {
    let encoded_destination_name = URL_SAFE_NO_PAD.encode(destination_name);
    let encoded_application_name = URL_SAFE_NO_PAD.encode(application_space);
    let address_hash = address_hash
        .iter()
        .map(|c| c.to_string())
        .collect::<Vec<String>>()
        .join(",");

    format!(
        "rns://D/{version}/{encoded_destination_name}/{encoded_application_name}/{address_hash}//"
    )
}
