use anyhow::{Error, Result};
use candle_core::{Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::bert::{BertModel, Config, DTYPE};
use hf_hub::{Repo, RepoType, api::tokio::Api};
use notitia::DatabaseEmbedder;
use tokenizers::{PaddingParams, Tokenizer};

pub struct BertEmbedder {
    model: BertModel,
    tokenizer: Tokenizer,
    device: Device,
}

impl BertEmbedder {
    pub async fn new() -> Result<Self> {
        let device = best_device(0)?;

        let api = Api::new()?;
        let repo = api.repo(Repo::with_revision(
            "sentence-transformers/all-MiniLM-L6-v2".to_string(),
            RepoType::Model,
            "refs/pr/21".to_string(),
        ));

        let config_filename = repo.get("config.json").await?;
        let tokenizer_filename = repo.get("tokenizer.json").await?;
        let weights_filename = repo.get("model.safetensors").await?;

        let config = std::fs::read_to_string(config_filename)?;
        let mut config: Config = serde_json::from_str(&config)?;
        config.hidden_act = candle_transformers::models::bert::HiddenAct::GeluApproximate;

        let mut tokenizer = Tokenizer::from_file(tokenizer_filename).map_err(Error::msg)?;
        let vb =
            unsafe { VarBuilder::from_mmaped_safetensors(&[weights_filename], DTYPE, &device)? };

        let model = BertModel::load(vb, &config)?;

        if let Some(pp) = tokenizer.get_padding_mut() {
            pp.strategy = tokenizers::PaddingStrategy::BatchLongest;
        } else {
            let pp = PaddingParams {
                strategy: tokenizers::PaddingStrategy::BatchLongest,
                ..Default::default()
            };
            tokenizer.with_padding(Some(pp));
        }

        Ok(Self {
            model,
            tokenizer,
            device,
        })
    }

    fn embed_text(&self, text: &str) -> Result<Vec<f32>> {
        let tokens = self.tokenizer.encode(text, true).map_err(Error::msg)?;

        let token_ids = Tensor::new(tokens.get_ids(), &self.device)?.unsqueeze(0)?;
        let attention_mask =
            Tensor::new(tokens.get_attention_mask(), &self.device)?.unsqueeze(0)?;
        let token_type_ids = token_ids.zeros_like()?;

        let embeddings = self
            .model
            .forward(&token_ids, &token_type_ids, Some(&attention_mask))?;

        // Mean pooling over token dimension.
        let (_, seq_len, _) = embeddings.dims3()?;
        let embedding = (embeddings.sum(1)? / (seq_len as f64))?;
        let embedding = normalize_l2(&embedding)?;

        Ok(embedding.squeeze(0)?.to_vec1::<f32>()?)
    }
}

impl DatabaseEmbedder for BertEmbedder {
    fn embed(&self, text: &str) -> Vec<f32> {
        self.embed_text(text).expect("BertEmbedder::embed failed")
    }

    fn dimension(&self) -> u32 {
        384 // all-MiniLM-L6-v2 output dimension
    }
}

fn normalize_l2(v: &Tensor) -> candle_core::Result<Tensor> {
    v.broadcast_div(&v.sqr()?.sum_keepdim(1)?.sqrt()?)
}

fn best_device(#[allow(unused)] ordinal: usize) -> candle_core::Result<Device> {
    #[cfg(feature = "metal")]
    {
        if let Ok(dev) = Device::new_metal(ordinal) {
            return Ok(dev);
        }
    }

    #[cfg(feature = "cuda")]
    {
        if let Ok(dev) = Device::new_cuda(ordinal) {
            return Ok(dev);
        }
    }

    Ok(Device::Cpu)
}
