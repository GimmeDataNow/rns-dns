use std::collections::HashMap;
use std::default;

use chrono::DateTime;
use chrono::Utc;

use ed25519_dalek::Signature;
use rand_core::OsRng;
use reticulum::destination::Destination;
use reticulum::hash::AddressHash;
use reticulum::identity::Identity;
use x25519_dalek::PublicKey;

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};

pub const RECORD_EXPIRY: chrono::TimeDelta = chrono::Duration::days(365);

#[derive(Debug, Clone)]
pub enum Connection {
    Tcp {
        local_host: String,
        local_port: u16,
    },
    Udp {
        local_host: String,
        local_port: u16,
        remote_host: String,
        remote_port: u16,
    },
    // For LoRa/Serial add:
    Serial {
        path: String,
        baud: u32,
    },
    LoRa {
        freq: u32,
        bw: u32,
        sf: u8,
        tx_power: u8,
    },
    // You can add Tor/I2P/RNode/etc later
}
impl ToString for Connection {
    fn to_string(&self) -> String {
        match &self {
            Self::Tcp {
                local_host,
                local_port,
            } => format!("Tcp;{local_host}:{local_port}"),
            Self::Udp {
                local_host,
                local_port,
                remote_host,
                remote_port,
            } => format!("Udp;{local_host}:{local_port}:{remote_host}:{remote_port}"),
            Self::Serial { path, baud } => format!("Serial;{path}:{baud}"),
            Self::LoRa {
                freq,
                bw,
                sf,
                tx_power,
            } => format!("LoRa;{freq}:{bw}:{sf}:{tx_power}"),
            _ => todo!(),
        }
    }
}

impl Connection {
    pub fn new_tcp(local_host: String, local_port: u16) -> Self {
        Self::Tcp {
            local_host,
            local_port,
        }
    }
    pub fn new_udp(
        local_host: String,
        local_port: u16,
        remote_host: String,
        remote_port: u16,
    ) -> Self {
        Self::Udp {
            local_host,
            local_port,
            remote_host,
            remote_port,
        }
    }
}

pub enum PrivateIdentity {
    FromString(String),
    FromHexString(String),
    Rand,
}
impl Default for PrivateIdentity {
    fn default() -> Self {
        Self::Rand
    }
}

impl PrivateIdentity {
    pub fn extract(&self) -> reticulum::identity::PrivateIdentity {
        match &self {
            PrivateIdentity::Rand => reticulum::identity::PrivateIdentity::new_from_rand(OsRng),
            PrivateIdentity::FromString(s) => {
                reticulum::identity::PrivateIdentity::new_from_name(&s)
            }
            PrivateIdentity::FromHexString(s) => {
                reticulum::identity::PrivateIdentity::new_from_hex_string(&s)
                    .expect("failed to convert hex string to private identity")
            }
        }
    }
}

pub struct DestinationConfig {
    pub app_name: String,
    pub application_space: String,
}

impl DestinationConfig {
    pub fn new(app_name: String, application_space: String) -> Self {
        Self {
            app_name,
            application_space,
        }
    }
}

pub struct NodeSettings {
    pub interfaces: Vec<Connection>,
    pub private_identity: PrivateIdentity,
}

impl NodeSettings {
    pub fn new(interfaces: Vec<Connection>, private_identity: PrivateIdentity) -> Self {
        Self {
            interfaces,
            private_identity,
        }
    }
}

pub fn generate_node_url(
    version: &u16,
    address_hash: &Vec<AddressHash>,
    public_key: &PublicKey,
    interfaces: &Vec<Connection>,
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

/// This is a single dns entry. It serves to provide the destination, public key and
/// validity of an entry.
///
/// # Fields
/// `name` - The name is simply the human readable domain name.
/// `destination` - The destination the domain points to.
/// `public_key` - The public key from which the destination is derived from. It is
/// also used in verification of the record.
/// `timestamp` - The timestamp at which the record was last updated.
/// `expiry` - The timestamp at which the record will cease to be valid.
/// `signature` - The signature to validate the record.
/// `verification` - The list of verifiers that have vouched for this node.
///
/// # Reasoning
///
/// This struct contains both the destination and public key as seperate entries
/// even though the destination can be derived from the public key. This is because
/// It would be unnecessary cpu work to recompute it for every user and should the
/// client forgo any validation then this will enable it to route quickly.
///
/// The struct provides both the last time it was updated and when the record will
/// expire. Record are generally going to live for a very long time since there is
/// little reason for a record to invalidate. This does NOT make any statements in
/// regards to routing.  
///
/// The signature is provided to validate the record and its contents.
///
/// The timestamp and expiry should only be controlled by the dns server. The node
/// that submits the dns-entry-addition-request may request for a certain expiry
/// but this request should be strictly reviewed by the server.
///
/// # Security
///
/// The entry relies on the 'submitter' (the node that request to be placed into the
/// dns records) to validate and provide a public signature that is derived from the
/// private key and the contents that will be signed.
#[allow(dead_code)]
#[derive(Clone)]
pub struct DnsEntry {
    /// The name is simply the human readable domain name.
    name: String,
    /// The destination the domain points to.
    destinations: Vec<AddressHash>,
    /// The public key from which the destination is derived from. It is
    /// also used in verification of the record.
    public_key: PublicKey,
    /// The timestamp at which the record was last updated.
    timestamp: DateTime<Utc>,
    /// The timestamp at which the record will cease to be valid.
    expiry: DateTime<Utc>,
    /// The signature to validate the record.
    signature: Signature,
    /// The list of verifiers that have vouched for this node.
    ///
    /// This should be sorted by the server according to trust levels.
    verifications: Vec<VerifierSigning>,
}

impl Default for DnsEntry {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            name: "example.com".into(),
            destinations: vec![],
            // public_key: String::default(),
            public_key: todo!(),

            timestamp: now,
            expiry: now + RECORD_EXPIRY,
            // signature: String::default(),
            signature: todo!(),

            verifications: vec![],
        }
    }
}

impl DnsEntry {
    pub fn is_entry_expired(&self) -> bool {
        self.expiry > Utc::now()
    }

    pub fn update_timestamp(&mut self, timestamp: DateTime<Utc>) -> &mut Self {
        self.timestamp = timestamp;
        self
    }
}

/// This is the the signing of a verifier. It contains minimal information. Should
/// more Information be required then a look-up using the `destination` is to be
/// made.
///
/// # Fields
/// `destination` - The 'id' of the verifier and the callback destination for
/// revalidation.
/// `signature` - The signature validating the dns entry.
///
/// # Reasoning
///
/// This struct only contains the destination and the corresponding signature. This
/// is because it is expected that the client will perform a look-up on the
/// destination and cache the results in a long term database. The contents of this
/// cache are unlikely to change often and not repeatedly sending this data will
/// greatly reduce the data being sent.
///
/// # Security
///
/// The only important security note to make is that the signature is derived from
/// the signature of the dns entry along with the private key of the verifier.
///
/// The public key corresponding to this signature will have to be retrieved from
/// another call.
#[allow(dead_code)]
#[derive(Clone)]
pub struct VerifierSigning {
    /// The destination of the verifier
    destination: AddressHash,
    /// The signature validating the dns entry
    signature: Signature, // sig(dnsentry.sig(), verifier.destination)
}

/// This is a representation of a verification authority, a so called `verifier`.
///
/// # Fields
/// `name` - The human readable name of the verifying authority.
/// `destination` - The destination of the verifying authority and the callback
/// endpoint for dns entry modifications.
/// `trust_level` - The trust level of the verifying authority.
/// `public_key` - The public key which is used for signatue validation.
///
/// # Reasoning
///
/// This struct contains detailed data about the verifying authority and it provides
/// all necessary data for validation of signatures.
///
/// The name, trust levels and public keys are only presented here in order to
/// reduce amount of the data that needs to be transfered. This data is long lived
/// in nature and this data should be cached for long periods of time and thus this
/// data should live seperately to the rest of the dns data.
///
/// Verifiers are selected by the dns server and each of them posses a trust level.
/// These trust levels are non unique numbers where `0` represents the absolute
/// highest level of trust that a dns server can issue and this level is reserved
/// for the dns server itself. The trust level should be an inverse representation
/// the actual level of trust that you place in them i.e. the trust level is more
/// akin to the level of risk that that node may pose.
///
/// The (human-readable) name should be available for those that wish to research
/// more about the verifying authority.
///
/// # Security
///
/// Only the dns server should have the ability to modify these records and NOT the
/// trusted authorites. The reason is because is is a highly sensitive record which
/// could cause cascading consequences if improperly handled.
#[allow(dead_code)]
pub struct Verifier {
    /// The human-readable name of the verifier
    name: String,
    /// The destination of the verifier and the callback destination for dns
    /// modifications.
    destination: AddressHash,
    /// The trust level of the verifier. `0` represents the highest level.
    trust_level: u32,
    /// The public key used for validating signatures
    public_key: PublicKey,
}

pub enum RNSDNSERRORS {
    AlreadyExists,
}

/// This is the DnsDatabase.
///
/// # Fields
/// `entries` - This is the forward index which maps from the String to the DnsEntry
/// `reverse_index` - This is the reverse index which maps from the Destination to
/// the domain names
///
/// # Reasoning
///
/// I belive that it would be best to keep the index and the reverse index in one
/// struct since they often affect each other and since they are already closely
/// related.
///
/// Other aspect of the dns server will live in other structs.
///
/// # Security
///
/// This struct byitself will not offer any security features. These are to be
/// handled by the function caller in a responsible manner.
#[derive(Default)]
pub struct DnsEntryStore {
    forward_index: HashMap<String, DnsEntry>,
    reverse_index: HashMap<AddressHash, Vec<String>>,
}

impl DnsEntryStore {
    /// Adds an entry to the `DnsEntryStore`.
    ///
    /// # Behaviour
    ///
    /// THIS FUNCTION DOES NOT UPDATE THE REVERSE INDEX.
    ///
    /// It will default for all values that were not specified such as `timestamp`,
    /// `expiry`, `verifications`.
    /// Other fields will be constructed using the data that was supplied, keeping
    /// them minimally functional.
    ///
    /// # Errors
    ///
    /// Should this record already exist in the forward index then this function will
    /// return `RNSDNSERRORS::AlreadyExists`.
    pub fn add_entry(
        &mut self,
        name: &String,
        destination: &AddressHash,
        public_key: &PublicKey,
        signature: Signature,
    ) -> Result<(), RNSDNSERRORS> {
        //
        if self.forward_index.contains_key(name) {
            return Err(RNSDNSERRORS::AlreadyExists);
        }

        // get the domain names and if there are none just default to an empty vec
        // let domain_names = self.reverse_index.entry(*destination).or_default();

        // add the domain name if it is not already present
        // if !domain_names.contains(name) { domain_names.push(name.clone()); }

        let now = Utc::now();

        // error if this domain name already exists

        let _ = self.forward_index.insert(
            name.clone(),
            DnsEntry {
                name: name.clone(),
                destinations: vec![*destination],
                public_key: public_key.clone(),
                timestamp: now,
                expiry: now + RECORD_EXPIRY,
                signature,
                verifications: Vec::default(),
            },
        );

        Ok(())
    }

    /// Forcefully overrides an entry ignoring any restrictions.
    ///
    /// # Behaviour
    ///
    /// THIS FUNCTION DOES NOT UPDATE THE REVERSE INDEX.
    ///
    /// It first searches for the entry using the name from the entry which it then
    /// overrides in its entirety
    pub fn override_entry(&mut self, entry: DnsEntry) {
        let a = self.forward_index.entry(entry.name.clone()).or_default();
        *a = entry;
    }

    /// Remove an entry from the forward index
    pub fn remove_domain(&mut self, domain: &str) {
        self.forward_index.remove(domain);
    }

    /// Returns the DnsEntry for a given domain name should one exist.
    ///
    /// # Behaviour
    ///
    /// This function will return `None` if there is no entry.
    pub fn lookup(&self, name: &str) -> Option<&DnsEntry> {
        self.forward_index.get(name)
    }

    /// Returns the list of domain names which are associated with this destination.
    ///
    /// # Behaviour
    ///
    /// This function will return `None` if there is no entry. It might also return
    /// an empty list.
    pub fn reverse_lookup(&self, destination: &AddressHash) -> Option<&Vec<String>> {
        self.reverse_index.get(destination)
    }

    /// Completely rebuilds the entire reverse index
    ///
    pub fn rebuild_reverse_index(&mut self) {
        // delete all previous records
        self.reverse_index = HashMap::default();

        // iter over every known dnsentry
        for (domain, entry) in &self.forward_index {
            // for every known dest add them to the reverse index if it ins't
            // already present.
            for dest in &entry.destinations {
                let entry = self.reverse_index.entry(dest.clone()).or_default();
                if !entry.contains(domain) {
                    entry.push(domain.clone());
                }
            }
        }
    }

    /// Returns all of the forward index entries.
    pub fn get_active_forward_index(&self) -> Vec<DnsEntry> {
        self.forward_index.values().cloned().collect()
    }

    pub fn list_all_domain(&self) {
        todo!()
    }
}

#[allow(dead_code)]
#[derive(Default)]
pub struct VerificationStore {
    verifier_signings: HashMap<(String, AddressHash), Vec<VerifierSigning>>,
}

#[allow(unused_variables)]
impl VerificationStore {
    pub fn add_verification(
        &mut self,
        name: String,
        destination: AddressHash,
        signature: VerifierSigning,
    ) -> Result<(), RNSDNSERRORS> {
        // TODO: verify the actual signature before doing anything
        // maybe call on verify_verifier_signature()
        todo!()
    }
    pub fn verify_verifier_signature(&self) -> bool {
        todo!()
    }

    pub fn get_domains_by_verifier(&self, verifier: Verifier) -> Vec<&DnsEntry> {
        // will require additional args
        // this might be really hard to keep clean

        todo!()
    }
    pub fn count_verifications(&self, name: String, destination: AddressHash) -> u32 {
        todo!()
    }
    pub fn get_verifications_for_domain(
        &self,
        name: String,
        destination: AddressHash,
    ) -> Result<&Vec<VerifierSigning>, RNSDNSERRORS> {
        todo!()
    }
}

#[derive(Default)]
pub struct VerifierRegistry {
    verifiers: HashMap<AddressHash, Verifier>,
}

impl VerifierRegistry {
    pub fn add_verifier() {
        todo!()
    }
    pub fn get_verifier() {
        todo!()
    }
    pub fn get_entries_by_trust_level() {
        todo!()
    }
    pub fn get_entries_by_verifier() {
        todo!()
    }
    pub fn is_entry_trusted() {
        todo!()
    }
}

#[derive(Default)]
pub struct DnsDatabase {
    entry_store: DnsEntryStore,
    verification_store: VerificationStore,
    verifier_registry: VerifierRegistry,
}
impl DnsDatabase {}
