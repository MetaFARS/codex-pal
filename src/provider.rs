use anyhow::{Result, bail};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProviderModel {
    pub slug: &'static str,
    pub display_name: &'static str,
    pub description: &'static str,
    pub context_window: u32,
}

pub const BUILTIN_PROVIDERS: &[&str] = &[
    "openai",
    "deepseek",
    "z",
    "zai",
    "kimi",
    "moonshot",
    "qwen",
    "dashscope",
    "mistral",
    "groq",
    "xai",
    "grok",
    "openrouter",
    "custom",
];

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
        "z" | "zai" => Ok(("https://api.z.ai/api/paas/v4", "ZAI_API_KEY")),
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

pub fn is_builtin_provider(name: &str) -> bool {
    let normalized = name.trim().to_ascii_lowercase();
    provider_defaults(&normalized).is_ok()
}

pub fn default_model(provider: &str) -> Option<&'static str> {
    provider_models(provider)
        .first()
        .map(|model| model.slug)
        .or_else(|| match provider.trim().to_ascii_lowercase().as_str() {
            "openai" => Some("gpt-5.5"),
            _ => None,
        })
}

pub fn provider_models(provider: &str) -> &'static [ProviderModel] {
    match provider.trim().to_ascii_lowercase().as_str() {
        "deepseek" => &DEEPSEEK_MODELS,
        "z" | "zai" => &ZAI_MODELS,
        "kimi" | "moonshot" => &KIMI_MODELS,
        "qwen" | "dashscope" => &QWEN_MODELS,
        "mistral" => &MISTRAL_MODELS,
        "groq" => &GROQ_MODELS,
        "xai" | "grok" => &XAI_MODELS,
        "openrouter" => &OPENROUTER_MODELS,
        _ => &[],
    }
}

const DEEPSEEK_MODELS: [ProviderModel; 2] = [
    ProviderModel {
        slug: "deepseek-v4-pro",
        display_name: "DeepSeek-V4-Pro",
        description: "DeepSeek frontier model with thinking mode, 1M context, JSON output, and tool calls.",
        context_window: 1_000_000,
    },
    ProviderModel {
        slug: "deepseek-v4-flash",
        display_name: "DeepSeek-V4-Flash",
        description: "DeepSeek faster V4 model with thinking mode, 1M context, JSON output, and tool calls.",
        context_window: 1_000_000,
    },
];

const ZAI_MODELS: [ProviderModel; 1] = [ProviderModel {
    slug: "glm-5.2",
    display_name: "GLM-5.2",
    description: "Z.ai's flagship model for coding and long-horizon tasks with configurable reasoning effort.",
    context_window: 1_000_000,
}];

const KIMI_MODELS: [ProviderModel; 4] = [
    ProviderModel {
        slug: "kimi-k3",
        display_name: "Kimi K3",
        description: "Kimi's flagship model for long-horizon coding, knowledge work, and reasoning.",
        context_window: 1_000_000,
    },
    ProviderModel {
        slug: "kimi-k2.7-code",
        display_name: "Kimi K2.7 Code",
        description: "Kimi's strongest coding model for agentic coding workflows.",
        context_window: 256_000,
    },
    ProviderModel {
        slug: "kimi-k2.7-code-highspeed",
        display_name: "Kimi K2.7 Code Highspeed",
        description: "High-speed Kimi K2.7 Code variant with the same model parameters.",
        context_window: 256_000,
    },
    ProviderModel {
        slug: "kimi-k2.6",
        display_name: "Kimi K2.6",
        description: "Kimi general chat and multimodal model.",
        context_window: 256_000,
    },
];

const QWEN_MODELS: [ProviderModel; 3] = [
    ProviderModel {
        slug: "qwen3.7-max",
        display_name: "Qwen3.7 Max",
        description: "DashScope's strongest Qwen text-generation model.",
        context_window: 1_000_000,
    },
    ProviderModel {
        slug: "qwen3.7-plus",
        display_name: "Qwen3.7 Plus",
        description: "DashScope high-capability Qwen text-generation model.",
        context_window: 1_000_000,
    },
    ProviderModel {
        slug: "qwen3.6-flash",
        display_name: "Qwen3.6 Flash",
        description: "DashScope low-latency Qwen text-generation model.",
        context_window: 1_000_000,
    },
];

const MISTRAL_MODELS: [ProviderModel; 3] = [
    ProviderModel {
        slug: "mistral-medium-3-5+2",
        display_name: "Mistral Medium 3.5",
        description: "Mistral frontier multimodal model optimized for agentic and coding use cases.",
        context_window: 256_000,
    },
    ProviderModel {
        slug: "mistral-small-2603+1",
        display_name: "Mistral Small 4",
        description: "Mistral hybrid model unifying instruct, reasoning, and coding capabilities.",
        context_window: 256_000,
    },
    ProviderModel {
        slug: "devstral-2512",
        display_name: "Devstral 2",
        description: "Mistral code agents model for software engineering tasks.",
        context_window: 256_000,
    },
];

const GROQ_MODELS: [ProviderModel; 4] = [
    ProviderModel {
        slug: "openai/gpt-oss-120b",
        display_name: "GPT OSS 120B",
        description: "OpenAI's flagship open-weight model hosted by Groq.",
        context_window: 131_072,
    },
    ProviderModel {
        slug: "groq/compound",
        display_name: "Groq Compound",
        description: "Groq agentic system that can use built-in tools such as web search and code execution.",
        context_window: 131_072,
    },
    ProviderModel {
        slug: "llama-3.3-70b-versatile",
        display_name: "Llama 3.3 70B Versatile",
        description: "Groq production Llama 3.3 70B model.",
        context_window: 131_072,
    },
    ProviderModel {
        slug: "qwen/qwen3.6-27b",
        display_name: "Qwen3.6 27B",
        description: "Qwen3.6 preview model hosted by Groq.",
        context_window: 131_072,
    },
];

const XAI_MODELS: [ProviderModel; 2] = [
    ProviderModel {
        slug: "grok-4.3",
        display_name: "Grok 4.3",
        description: "xAI's recommended general-purpose model with agentic tool calling.",
        context_window: 1_000_000,
    },
    ProviderModel {
        slug: "grok-build-0.1",
        display_name: "Grok Build 0.1",
        description: "xAI coding model trained for agentic coding workflows.",
        context_window: 256_000,
    },
];

const OPENROUTER_MODELS: [ProviderModel; 1] = [ProviderModel {
    slug: "openrouter/auto",
    display_name: "OpenRouter Auto",
    description: "OpenRouter router that automatically selects a suitable model for each prompt.",
    context_window: 128_000,
}];

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
    fn resolves_zai_aliases_and_current_default_models() {
        for provider in ["z", "zai"] {
            let args = crate::cli::ProviderArgs {
                provider: provider.to_string(),
                upstream: None,
                api_key_env: None,
                api_key: None,
            };
            let profile = ProviderProfile::resolve(&args).unwrap();
            assert_eq!(profile.upstream, "https://api.z.ai/api/paas/v4");
            assert_eq!(profile.api_key_env, "ZAI_API_KEY");
            assert_eq!(default_model(provider), Some("glm-5.2"));
        }
        assert_eq!(default_model("deepseek"), Some("deepseek-v4-pro"));
        assert_eq!(default_model("kimi"), Some("kimi-k3"));
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
