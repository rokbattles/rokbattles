use crate::DrastcCategories;

const WEIGHTS_TABLE: &[WeightedCategory] = &[
    WeightedCategory::new(Category::Damage, 0.25),
    WeightedCategory::new(Category::Rage, 0.20),
    WeightedCategory::new(Category::Assist, 0.10),
    WeightedCategory::new(Category::Sustainability, 0.20),
    WeightedCategory::new(Category::Trade, 0.15),
    WeightedCategory::new(Category::Consistency, 0.10),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Category {
    Damage,
    Rage,
    Assist,
    Sustainability,
    Trade,
    Consistency,
}

impl Category {
    fn score(self, categories: &DrastcCategories) -> f64 {
        match self {
            Self::Damage => categories.damage.score,
            Self::Rage => categories.rage.score,
            Self::Assist => categories.assist.score,
            Self::Sustainability => categories.sustainability.score,
            Self::Trade => categories.trade.score,
            Self::Consistency => categories.consistency.score,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct WeightedCategory {
    category: Category,
    weight: f64,
}

impl WeightedCategory {
    const fn new(category: Category, weight: f64) -> Self {
        Self { category, weight }
    }
}

pub(crate) fn weighted_overall(categories: &DrastcCategories) -> f64 {
    WEIGHTS_TABLE.iter().map(|entry| entry.category.score(categories) * entry.weight).sum()
}
