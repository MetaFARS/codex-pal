use anyhow::{Result, bail};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderProfile {
    pub name: String,
    pub upstream: String,
    pub api_key_env: String,
    pub api_key: Option<String>,
}

impl ProviderProfile {
    pub fn resolve(args: &crate::cli::ProviderArgs) -> Result<Self> {
        let provider = args.provider.trim().to_ascii_lowercase();
        let (default_upstream, default_key_env) = provider_defaults(&provider)?;
        Ok(Self {
            name: provider,
            upstream: args
                .upstream
                .clone()
                .unwrap_or_else(|| default_upstream.to_string()),
            api_key_env: args
                .api_key_env
                .clone()
                .unwrap_or_else(|| default_key_env.to_string()),
            api_key: args.api_key.clone(),
        })
    }

    pub fn needs_relay(&self) -> bool {
        self.name != "openai"
    }

    pub fn api_key_value(&self) -> Option<String> {
        self.api_key
            .clone()
            .or_else(|| std::env::var(&self.api_key_env).ok())
    }
}

fn provider_defaults(provider: &str) -> Result<(&'static str, &'static str)> {
    match provider {
        "openai" => Ok(("https://api.openai.com/v1", "OPENAI_API_KEY")),
        "deepseek" => Ok(("https://api.deepseek.com/v1", "DEEPSEEK_API_KEY")),
        "kimi" | "moonshot" => Ok(("https://api.moonshot.cn/v1", "MOONSHOT_API_KEY")),
        "qwen" | "dashscope" => Ok((
            "https://dashscope.aliyuncs.com/compatible-mode/v1",
            "DASHSCOPE_API_KEY",
        )),
        "mistral" => Ok(("https://api.mistral.ai/v1", "MISTRAL_API_KEY")),
        "groq" => Ok(("https://api.groq.com/openai/v1", "GROQ_API_KEY")),
        "xai" | "grok" => Ok(("https://api.x.ai/v1", "XAI_API_KEY")),
        "openrouter" => Ok(("https://openrouter.ai/api/v1", "OPENROUTER_API_KEY")),
        "custom" => Ok(("https://openrouter.ai/api/v1", "OPENAI_API_KEY")),
        other => {
            bail!("unknown provider {other:?}; use --upstream/--api-key-env with --provider custom")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_known_provider() {
        let args = crate::cli::ProviderArgs {
            provider: "deepseek".to_string(),
            upstream: None,
            api_key_env: None,
            api_key: None,
        };
        let profile = ProviderProfile::resolve(&args).unwrap();
        assert_eq!(profile.upstream, "https://api.deepseek.com/v1");
        assert_eq!(profile.api_key_env, "DEEPSEEK_API_KEY");
        assert!(profile.needs_relay());
    }

    #[test]
    fn custom_can_override_upstream_and_key_env() {
        let args = crate::cli::ProviderArgs {
            provider: "custom".to_string(),
            upstream: Some("https://llm.example/v1".to_string()),
            api_key_env: Some("EXAMPLE_API_KEY".to_string()),
            api_key: None,
        };
        let profile = ProviderProfile::resolve(&args).unwrap();
        assert_eq!(profile.upstream, "https://llm.example/v1");
        assert_eq!(profile.api_key_env, "EXAMPLE_API_KEY");
    }
}
