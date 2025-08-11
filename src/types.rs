use std::collections::HashMap;

use chrono::DateTime;
use chrono::Utc;

use reticulum::destination::Destination;
use reticulum::hash::AddressHash;

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
pub struct DnsEntry {
    /// The name is simply the human readable domain name.
    name: String,
    /// The destination the domain points to.
    destinations: Vec<AddressHash>,
    /// The public key from which the destination is derived from. It is
    /// also used in verification of the record.
    public_key: String,
    /// The timestamp at which the record was last updated.
    timestamp: DateTime<Utc>,
    /// The timestamp at which the record will cease to be valid.
    expiry: DateTime<Utc>,
    /// The signature to validate the record.
    signature: String,
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
            public_key: String::default(),
            timestamp: now,
            expiry: now + RECORD_EXPIRY,
            signature: String::default(),
            verifications: vec![],
        }
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
pub struct VerifierSigning {
    /// The destination of the verifier
    destination: AddressHash,
    /// The signature validating the dns entry
    signature: String, // sig(dnsentry.sig(), verifier.destination)
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
pub struct Verifier {
    /// The human-readable name of the verifier 
    name: String,
    /// The destination of the verifier and the callback destination for dns
    /// modifications. 
    destination: AddressHash,
    /// The trust level of the verifier. `0` represents the highest level.
    trust_level: u32,
    /// The public key used for validating signatures
    public_key: String,
}


pub enum RNSDNSERRORS {
    AlreadyExists
}

#[derive(Default)]
pub struct DnsDatabase {
    pub entries: HashMap<String, DnsEntry>,
    pub reverse_index: HashMap<AddressHash,Vec<String>>,
    // (domain name, destination) both need to be specified because a single
    // signature only verfies one domain and one destination
    pub verifiers: HashMap<(String, AddressHash), VerifierSigning>,
}

pub const RECORD_EXPIRY: chrono::TimeDelta = chrono::Duration::days(365);


impl DnsDatabase {

    pub fn new() -> Self {
       Self::default()
    }
    
    // name, dest, public key, signature
    pub fn add_entry_unvalidated(
        &mut self,
        name: &String,
        destination: &AddressHash,
        public_key: &String,
        signature: String,
    ) -> Result<(), RNSDNSERRORS> {

        // get the domain names and if there are none just default to an empty vec
        let domain_names = self.reverse_index.entry(*destination).or_default();

        // add the domain name if it is not already present
        if !domain_names.contains(name) { domain_names.push(name.clone()); }

        let now = Utc::now();

        // error if this domain name already exists
        if self.entries.contains_key(name) { return Err(RNSDNSERRORS::AlreadyExists) }

        let _ = self.entries.insert(name.clone(), DnsEntry {
            name: name.clone(),
            destinations: vec![*destination],
            public_key: public_key.clone(),
            timestamp: now,
            expiry: now + RECORD_EXPIRY,
            signature,
            verifications: Vec::default(),
        });

        Ok(())
    }

    pub fn remove_domain(&mut self, domain: &str) {
        // remove the domain from the main index
        if let Some(entry) = self.entries.remove(domain) {
            // remove every known destination
            for destination in entry.destinations {
                self.reverse_index.remove(&destination);
            }
        }
    }

    pub fn loopup(&self, name: &String) -> Option<DnsEntry> {
        todo!()
    }

    pub fn reverse_lookup(&self, destination: &AddressHash) -> Option<&Vec<String>> {
        self.reverse_index.get(destination)
    }
    
}

