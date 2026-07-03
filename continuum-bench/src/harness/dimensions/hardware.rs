//! Hardware profile labels from EXPERIMENTS.md.

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

/// Hardware profile label from EXPERIMENTS.md.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum Hardware {
    #[value(name = "dev-wsl")]
    DevWsl,
    #[value(name = "ci-small")]
    CiSmall,
    #[value(name = "bare-metal-small")]
    BareMetalSmall,
    #[value(name = "bare-metal-medium")]
    BareMetalMedium,
    #[value(name = "bare-metal-large")]
    BareMetalLarge,
    #[value(name = "aws-t3-medium")]
    AwsT3Medium,
    #[value(name = "aws-t3-small")]
    AwsT3Small,
    #[value(name = "aws-t4g-small")]
    AwsT4gSmall,
    #[value(name = "aws-t4g-medium")]
    AwsT4gMedium,
    #[value(name = "aws-t4g-large")]
    AwsT4gLarge,
    #[value(name = "aws-c7i-4xlarge")]
    AwsC7i4xlarge,
    #[value(name = "aws-i4i-xlarge")]
    AwsI4iXlarge,
}

impl Hardware {
    /// Short slug for report filenames.
    pub const fn slug(self) -> &'static str {
        match self {
            Self::DevWsl => "dev-wsl",
            Self::CiSmall => "ci-small",
            Self::BareMetalSmall => "bare-metal-small",
            Self::BareMetalMedium => "bare-metal-medium",
            Self::BareMetalLarge => "bare-metal-large",
            Self::AwsT3Medium => "aws-t3-medium",
            Self::AwsT3Small => "aws-t3-small",
            Self::AwsT4gSmall => "aws-t4g-small",
            Self::AwsT4gMedium => "aws-t4g-medium",
            Self::AwsT4gLarge => "aws-t4g-large",
            Self::AwsC7i4xlarge => "aws-c7i-4xlarge",
            Self::AwsI4iXlarge => "aws-i4i-xlarge",
        }
    }

    /// Cloud / isolated-VM sizing profiles capture per-run CPU/RSS; lab `dev-wsl` is sanity-only.
    pub const fn captures_run_resource_profile(self) -> bool {
        matches!(
            self,
            Self::CiSmall
                | Self::AwsT3Medium
                | Self::AwsT3Small
                | Self::AwsT4gSmall
                | Self::AwsT4gMedium
                | Self::AwsT4gLarge
                | Self::AwsC7i4xlarge
                | Self::AwsI4iXlarge
        )
    }

    /// On-demand Linux hourly rate (USD) for fleet cost projection — us-west-2 approximate.
    pub const fn hourly_usd(self) -> f64 {
        match self {
            Self::DevWsl | Self::CiSmall => 0.0,
            Self::BareMetalSmall => 0.05,
            Self::BareMetalMedium => 0.10,
            Self::BareMetalLarge => 0.25,
            Self::AwsT3Small => 0.0208,
            Self::AwsT3Medium => 0.0416,
            Self::AwsT4gSmall => 0.0168,
            Self::AwsT4gMedium => 0.0336,
            Self::AwsT4gLarge => 0.0672,
            Self::AwsC7i4xlarge => 0.816,
            Self::AwsI4iXlarge => 0.312,
        }
    }

    /// Parse a hardware slug from report JSON or CLI strings.
    pub fn from_slug(s: &str) -> Option<Self> {
        match s {
            "dev-wsl" => Some(Self::DevWsl),
            "ci-small" => Some(Self::CiSmall),
            "bare-metal-small" => Some(Self::BareMetalSmall),
            "bare-metal-medium" => Some(Self::BareMetalMedium),
            "bare-metal-large" => Some(Self::BareMetalLarge),
            "aws-t3-medium" => Some(Self::AwsT3Medium),
            "aws-t3-small" => Some(Self::AwsT3Small),
            "aws-t4g-small" => Some(Self::AwsT4gSmall),
            "aws-t4g-medium" => Some(Self::AwsT4gMedium),
            "aws-t4g-large" => Some(Self::AwsT4gLarge),
            "aws-c7i-4xlarge" => Some(Self::AwsC7i4xlarge),
            "aws-i4i-xlarge" => Some(Self::AwsI4iXlarge),
            _ => None,
        }
    }
}
