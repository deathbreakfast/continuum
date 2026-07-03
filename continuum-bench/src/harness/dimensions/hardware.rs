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
    pub fn slug(self) -> &'static str {
        match self {
            Hardware::DevWsl => "dev-wsl",
            Hardware::CiSmall => "ci-small",
            Hardware::BareMetalSmall => "bare-metal-small",
            Hardware::BareMetalMedium => "bare-metal-medium",
            Hardware::BareMetalLarge => "bare-metal-large",
            Hardware::AwsT3Medium => "aws-t3-medium",
            Hardware::AwsT3Small => "aws-t3-small",
            Hardware::AwsT4gSmall => "aws-t4g-small",
            Hardware::AwsT4gMedium => "aws-t4g-medium",
            Hardware::AwsT4gLarge => "aws-t4g-large",
            Hardware::AwsC7i4xlarge => "aws-c7i-4xlarge",
            Hardware::AwsI4iXlarge => "aws-i4i-xlarge",
        }
    }

    /// Cloud / isolated-VM sizing profiles capture per-run CPU/RSS; lab `dev-wsl` is sanity-only.
    pub fn captures_run_resource_profile(self) -> bool {
        matches!(
            self,
            Hardware::CiSmall
                | Hardware::AwsT3Medium
                | Hardware::AwsT3Small
                | Hardware::AwsT4gSmall
                | Hardware::AwsT4gMedium
                | Hardware::AwsT4gLarge
                | Hardware::AwsC7i4xlarge
                | Hardware::AwsI4iXlarge
        )
    }

    /// On-demand Linux hourly rate (USD) for fleet cost projection — us-west-2 approximate.
    pub fn hourly_usd(self) -> f64 {
        match self {
            Hardware::DevWsl | Hardware::CiSmall => 0.0,
            Hardware::BareMetalSmall => 0.05,
            Hardware::BareMetalMedium => 0.10,
            Hardware::BareMetalLarge => 0.25,
            Hardware::AwsT3Small => 0.0208,
            Hardware::AwsT3Medium => 0.0416,
            Hardware::AwsT4gSmall => 0.0168,
            Hardware::AwsT4gMedium => 0.0336,
            Hardware::AwsT4gLarge => 0.0672,
            Hardware::AwsC7i4xlarge => 0.816,
            Hardware::AwsI4iXlarge => 0.312,
        }
    }

    /// Parse a hardware slug from report JSON or CLI strings.
    pub fn from_slug(s: &str) -> Option<Hardware> {
        match s {
            "dev-wsl" => Some(Hardware::DevWsl),
            "ci-small" => Some(Hardware::CiSmall),
            "bare-metal-small" => Some(Hardware::BareMetalSmall),
            "bare-metal-medium" => Some(Hardware::BareMetalMedium),
            "bare-metal-large" => Some(Hardware::BareMetalLarge),
            "aws-t3-medium" => Some(Hardware::AwsT3Medium),
            "aws-t3-small" => Some(Hardware::AwsT3Small),
            "aws-t4g-small" => Some(Hardware::AwsT4gSmall),
            "aws-t4g-medium" => Some(Hardware::AwsT4gMedium),
            "aws-t4g-large" => Some(Hardware::AwsT4gLarge),
            "aws-c7i-4xlarge" => Some(Hardware::AwsC7i4xlarge),
            "aws-i4i-xlarge" => Some(Hardware::AwsI4iXlarge),
            _ => None,
        }
    }
}
