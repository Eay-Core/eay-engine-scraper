use eay_scraper_core::error::Result;
use eay_scraper_core::recipe::Recipe;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// Watcher dan Registry Recipe yang membaca folder `recipes/`
/// Mendukung hot-reload tanpa compile ulang
#[derive(Clone)]
pub struct RecipeRegistry {
    dir_path: PathBuf,
    recipes: Arc<RwLock<HashMap<String, Recipe>>>,
}

impl RecipeRegistry {
    pub fn new<P: AsRef<Path>>(dir_path: P) -> Self {
        Self {
            dir_path: dir_path.as_ref().to_path_buf(),
            recipes: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Load semua file .yaml atau .yml di direktori recipes
    pub async fn load_all(&self) -> Result<usize> {
        if !self.dir_path.exists() {
            info!("Recipes directory {:?} does not exist yet", self.dir_path);
            return Ok(0);
        }

        let mut loaded = 0;
        let mut map = self.recipes.write().await;
        map.clear();

        let entries = std::fs::read_dir(&self.dir_path)?;
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension() {
                if ext == "yaml" || ext == "yml" {
                    match Recipe::from_file(&path) {
                        Ok(recipe) => {
                            info!("Loaded recipe: [{}] from {:?}", recipe.id, path);
                            map.insert(recipe.id.clone(), recipe);
                            loaded += 1;
                        }
                        Err(e) => {
                            warn!("Failed to load recipe from {:?}: {}", path, e);
                        }
                    }
                }
            }
        }

        info!("Total recipes loaded: {}", loaded);
        Ok(loaded)
    }

    /// Ambil recipe berdasarkan id provider
    pub async fn get_recipe(&self, id: &str) -> Option<Recipe> {
        let map = self.recipes.read().await;
        map.get(id).cloned()
    }

    /// Ambil semua recipes terdaftar
    pub async fn list_recipes(&self) -> Vec<Recipe> {
        let map = self.recipes.read().await;
        map.values().cloned().collect()
    }

    /// Simpan recipe baru atau modifikasi recipe ke folder dan memory
    pub async fn add_or_update_recipe(&self, recipe: Recipe) -> Result<()> {
        let yaml_str = recipe.to_yaml().map_err(|e| eay_scraper_core::error::ScraperError::Recipe(e.to_string()))?;
        let filename = format!("{}.yaml", recipe.id);
        let target_path = self.dir_path.join(filename);
        std::fs::write(&target_path, yaml_str)?;

        let mut map = self.recipes.write().await;
        map.insert(recipe.id.clone(), recipe);
        Ok(())
    }
}
