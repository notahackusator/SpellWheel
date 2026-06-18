use std::io::{Error, ErrorKind};
use fstools_dvdbnd::ArchiveKeyProvider;
use fstools_formats::bhd::BhdKey;

pub struct KeyProvider;

impl ArchiveKeyProvider for KeyProvider {
    fn get_key(&self, name: &str) -> Result<BhdKey, Error> {
        match name {
            "Data0" => BhdKey::from_pem(DATA0).map_err(Error::other),
            _ => Err(Error::new(ErrorKind::Unsupported, "Only Data0 is implemented for this key provider"))
        }
    }
}

const DATA0: &str = "-----BEGIN RSA PUBLIC KEY-----
MIIBCwKCAQEA9Rju2whruXDVQZpfylVEPeNxm7XgMHcDyaaRUIpXQE0qEo+6Y36L
P0xpFvL0H0kKxHwpuISsdgrnMHJ/yj4S61MWzhO8y4BQbw/zJehhDSRCecFJmFBz
3I2JC5FCjoK+82xd9xM5XXdfsdBzRiSghuIHL4qk2WZ/0f/nK5VygeWXn/oLeYBL
jX1S8wSSASza64JXjt0bP/i6mpV2SLZqKRxo7x2bIQrR1yHNekSF2jBhZIgcbtMB
xjCywn+7p954wjcfjxB5VWaZ4hGbKhi1bhYPccht4XnGhcUTWO3NmJWslwccjQ4k
sutLq3uRjLMM0IeTkQO6Pv8/R7UNFtdCWwIERzH8IQ==
-----END RSA PUBLIC KEY-----";