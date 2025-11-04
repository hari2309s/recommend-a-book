use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Query pattern types for template matching
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum QueryPattern {
    Author,
    Genre,
    Mood,
    SimilarTo,
    TimeBased,
    Audience,
    Length,
    Complexity,
    Theme,
    Award,
    Setting,
    Pace,
    Perspective,
    General,
}

/// Enhanced query information extracted from user input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedQuery {
    pub original_query: String,
    pub pattern: QueryPattern,
    pub extracted_terms: Vec<String>,
    pub expanded_terms: Vec<String>,
    pub filters: QueryFilters,
    pub search_hints: SearchHints,
}

/// Filters to apply during search
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QueryFilters {
    pub author: Option<String>,
    pub genres: Vec<String>,
    pub themes: Vec<String>,
    pub min_rating: Option<f32>,
    pub max_pages: Option<i32>,
    pub min_year: Option<i32>,
    pub max_year: Option<i32>,
    pub audience: Option<String>,
    pub settings: Vec<String>,
}

/// Hints for search strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHints {
    pub semantic_weight: f32,
    pub metadata_weight: f32,
    pub rating_boost: f32,
    pub recency_boost: f32,
}

impl Default for SearchHints {
    fn default() -> Self {
        Self {
            semantic_weight: 0.6,
            metadata_weight: 0.4,
            rating_boost: 1.0,
            recency_boost: 1.0,
        }
    }
}

lazy_static! {
    /// Common genre synonyms and expansions
    pub static ref GENRE_EXPANSIONS: HashMap<&'static str, Vec<&'static str>> = {
        let mut m = HashMap::new();
        m.insert("fantasy", vec!["fantasy", "epic fantasy", "high fantasy", "sword and sorcery", "magical realism", "urban fantasy", "dark fantasy"]);
        m.insert("sci-fi", vec!["science fiction", "sci-fi", "scifi", "space opera", "cyberpunk", "dystopian", "post-apocalyptic", "hard science fiction", "soft science fiction"]);
        m.insert("mystery", vec!["mystery", "detective", "crime", "thriller", "suspense", "whodunit", "noir", "cozy mystery", "police procedural"]);
        m.insert("romance", vec!["romance", "love story", "romantic", "contemporary romance", "historical romance", "romantic comedy", "paranormal romance"]);
        m.insert("horror", vec!["horror", "scary", "terror", "supernatural horror", "psychological horror", "gothic", "dark", "creepy"]);
        m.insert("historical", vec!["historical fiction", "historical", "period piece", "historical drama", "historical novel"]);
        m.insert("biography", vec!["biography", "memoir", "autobiography", "life story", "true story", "biographical"]);
        m.insert("self-help", vec!["self-help", "personal development", "self-improvement", "motivational", "psychology", "self care"]);
        m.insert("business", vec!["business", "entrepreneurship", "management", "leadership", "finance", "economics", "startup"]);
        m.insert("philosophy", vec!["philosophy", "philosophical", "ethics", "metaphysics", "existential", "epistemology"]);
        m.insert("young adult", vec!["young adult", "ya", "teen", "coming of age", "ya fiction", "teenage"]);
        m.insert("children", vec!["children", "kids", "juvenile", "picture book", "middle grade", "chapter book"]);
        m.insert("poetry", vec!["poetry", "poems", "verse", "poetic", "collection of poems"]);
        m.insert("drama", vec!["drama", "dramatic", "play", "theater", "theatrical"]);
        m.insert("adventure", vec!["adventure", "action", "quest", "journey", "expedition", "exploration"]);
        m.insert("literary", vec!["literary fiction", "literary", "contemporary fiction", "serious fiction", "literary novel"]);
        m.insert("thriller", vec!["thriller", "suspense", "action thriller", "spy thriller", "techno-thriller"]);
        m.insert("western", vec!["western", "wild west", "frontier", "cowboy"]);
        m.insert("satire", vec!["satire", "satirical", "parody", "social satire"]);
        m.insert("graphic novel", vec!["graphic novel", "comic", "manga", "comics", "illustrated novel"]);
        m.insert("true crime", vec!["true crime", "crime", "criminal", "murder case"]);
        m.insert("travel", vec!["travel", "travelogue", "travel writing", "journey"]);
        m.insert("cookbook", vec!["cookbook", "cooking", "recipes", "culinary"]);
        m.insert("spirituality", vec!["spirituality", "spiritual", "new age", "mindfulness", "meditation"]);
        m.insert("science", vec!["science", "popular science", "scientific", "physics", "biology", "chemistry", "astronomy"]);
        m.insert("history", vec!["history", "historical", "world history", "military history"]);
        m.insert("politics", vec!["politics", "political", "government", "political science"]);
        m.insert("art", vec!["art", "art history", "visual arts", "photography", "painting"]);
        m.insert("music", vec!["music", "musical", "music history", "music theory"]);
        m
    };

    /// Author name patterns
    pub static ref AUTHOR_PATTERNS: Vec<Regex> = vec![
        Regex::new(r"(?i)(?:books?\s+)?(?:written\s+)?by\s+([a-zA-Z\s.'-]+?)(?:\s+books?|\s+novels?|\s*$)").unwrap(),
        Regex::new(r"(?i)(?:works?\s+)?(?:of|from)\s+([a-zA-Z\s.'-]+?)(?:\s+books?|\s+novels?|\s*$)").unwrap(),
        Regex::new(r"(?i)([a-zA-Z\s.'-]+?)'s\s+(?:books?|novels?|works?|writings?)").unwrap(),
        Regex::new(r"(?i)author:?\s*([a-zA-Z\s.'-]+?)(?:\s|$)").unwrap(),
    ];

    /// Genre patterns
    pub static ref GENRE_PATTERNS: Vec<Regex> = vec![
        Regex::new(r"(?i)([a-zA-Z\s-]+?)\s+(?:books?|novels?|fiction|literature|stories)").unwrap(),
        Regex::new(r"(?i)(?:books?|novels?)\s+(?:in\s+)?([a-zA-Z\s-]+?)\s+(?:genre|category)").unwrap(),
        Regex::new(r"(?i)genre:?\s*([a-zA-Z\s-]+?)(?:\s|$)").unwrap(),
    ];

    /// Mood/atmosphere patterns
    pub static ref MOOD_PATTERNS: Vec<Regex> = vec![
        Regex::new(r"(?i)(?:feel|feeling|mood|atmosphere|vibe|tone)\s+(?:like\s+)?([a-zA-Z\s-]+)").unwrap(),
        Regex::new(r"(?i)\b(cozy|dark|light|uplifting|depressing|happy|sad|emotional|funny|humorous|serious|intense|relaxing|heartwarming|bittersweet|melancholic|optimistic|pessimistic|suspenseful|tense|peaceful|violent|gritty|whimsical|playful)\b").unwrap(),
    ];

    /// Similar-to patterns
    pub static ref SIMILAR_PATTERNS: Vec<Regex> = vec![
        Regex::new(r"(?i)(?:similar\s+to|like)\s+(.+)").unwrap(),
        Regex::new(r"(?i)(?:more|another)\s+(?:book|books)\s+like\s+(.+)").unwrap(),
        Regex::new(r"(?i)if\s+(?:I|you)\s+liked?\s+(.+)").unwrap(),
        Regex::new(r"(?i)reminds?\s+me\s+of\s+(.+)").unwrap(),
        Regex::new(r"(?i)in\s+the\s+style\s+of\s+(.+)").unwrap(),
    ];

    /// Time-based patterns
    pub static ref TIME_PATTERNS: Vec<Regex> = vec![
        Regex::new(r"(?i)\b(recent|new|latest|modern|contemporary|current|2020s|2010s)\b").unwrap(),
        Regex::new(r"(?i)\b(classic|old|vintage|timeless|traditional|golden age)\b").unwrap(),
        Regex::new(r"(?i)(?:published|released|written|from)\s+(?:in|around|after|before)\s+(\d{4})").unwrap(),
    ];

    /// Audience patterns
    pub static ref AUDIENCE_PATTERNS: Vec<Regex> = vec![
        Regex::new(r"(?i)(?:for\s+)?(?:kids|children|child)").unwrap(),
        Regex::new(r"(?i)(?:for\s+)?(?:teens?|teenagers?|young\s+adults?|ya)").unwrap(),
        Regex::new(r"(?i)(?:for\s+)?(?:adults?|grown-ups?|mature)").unwrap(),
    ];

    /// Length/pace patterns
    pub static ref LENGTH_PATTERNS: Vec<Regex> = vec![
        Regex::new(r"(?i)\b(short|quick|brief|concise|novella)\b").unwrap(),
        Regex::new(r"(?i)\b(long|lengthy|epic|extensive|saga|trilogy|series)\b").unwrap(),
    ];

    pub static ref PACE_PATTERNS: Vec<Regex> = vec![
        Regex::new(r"(?i)\b(fast[\s-]?paced|quick[\s-]?paced|action[\s-]?packed|thrilling|exciting)\b").unwrap(),
        Regex::new(r"(?i)\b(slow[\s-]?paced|slow[\s-]?burn|contemplative|meditative|leisurely)\b").unwrap(),
    ];

    pub static ref COMPLEXITY_PATTERNS: Vec<Regex> = vec![
        Regex::new(r"(?i)\b(easy|simple|light|accessible|beginner|straightforward|uncomplicated)\b").unwrap(),
        Regex::new(r"(?i)\b(complex|difficult|challenging|dense|deep|intellectual|advanced|sophisticated|cerebral)\b").unwrap(),
    ];

    /// Setting/location patterns
    pub static ref SETTING_PATTERNS: Vec<Regex> = vec![
        Regex::new(r"(?i)set\s+in\s+([a-zA-Z\s]+)").unwrap(),
        Regex::new(r"(?i)takes?\s+place\s+in\s+([a-zA-Z\s]+)").unwrap(),
        Regex::new(r"(?i)(?:located|based)\s+in\s+([a-zA-Z\s]+)").unwrap(),
    ];

    /// Perspective/POV patterns
    pub static ref PERSPECTIVE_PATTERNS: Vec<Regex> = vec![
        Regex::new(r"(?i)\b(first[\s-]?person|1st\s+person)\b").unwrap(),
        Regex::new(r"(?i)\b(third[\s-]?person|3rd\s+person)\b").unwrap(),
        Regex::new(r"(?i)\b(multiple\s+(?:pov|perspectives?|viewpoints?)|alternating\s+perspectives?)\b").unwrap(),
        Regex::new(r"(?i)\b(unreliable\s+narrator)\b").unwrap(),
    ];

    /// Theme keywords (significantly expanded)
    pub static ref THEME_KEYWORDS: HashMap<&'static str, Vec<&'static str>> = {
        let mut m = HashMap::new();

        // Relationships & Emotions
        m.insert("friendship", vec!["friendship", "friends", "companionship", "buddy", "camaraderie"]);
        m.insert("love", vec!["love", "romance", "relationship", "romantic", "passion"]);
        m.insert("family", vec!["family", "parent", "mother", "father", "sibling", "child", "familial"]);
        m.insert("betrayal", vec!["betrayal", "betrayed", "backstab", "treachery", "deception"]);
        m.insert("loss", vec!["loss", "grief", "mourning", "bereavement", "death of loved one"]);
        m.insert("redemption", vec!["redemption", "redemptive", "second chance", "forgiveness"]);

        // Conflict & Power
        m.insert("war", vec!["war", "battle", "conflict", "military", "soldier", "combat", "warfare"]);
        m.insert("politics", vec!["politics", "political", "government", "power", "corruption", "conspiracy"]);
        m.insert("revolution", vec!["revolution", "rebellion", "uprising", "revolt", "resistance"]);
        m.insert("revenge", vec!["revenge", "vengeance", "retribution", "payback"]);
        m.insert("murder", vec!["murder", "killing", "death", "assassination", "homicide"]);

        // Deception & Truth
        m.insert("lies", vec!["lies", "lying", "liar", "lie", "dishonesty", "falsehood", "untruth"]);
        m.insert("deception", vec!["deception", "deceive", "deceit", "deceiving", "trickery", "fraud", "manipulation"]);
        m.insert("secrets", vec!["secrets", "secret", "hidden", "concealed", "mystery"]);
        m.insert("truth", vec!["truth", "honesty", "revealing", "uncovering", "expose"]);

        // Fantasy & SciFi Elements
        m.insert("magic", vec!["magic", "magical", "wizard", "witch", "sorcery", "spell", "enchantment"]);
        m.insert("dragon", vec!["dragon", "dragons", "drake", "wyvern"]);
        m.insert("space", vec!["space", "galaxy", "planet", "spaceship", "star", "cosmos", "interstellar"]);
        m.insert("time-travel", vec!["time travel", "time machine", "temporal", "time loop"]);
        m.insert("artificial-intelligence", vec!["artificial intelligence", "a.i.", "robot", "android", "cyborg", "machine intelligence", "artificial-intelligence"]);
        m.insert("dystopia", vec!["dystopia", "dystopian", "apocalypse", "post-apocalyptic", "end of world"]);
        m.insert("utopia", vec!["utopia", "utopian", "perfect society", "ideal world"]);
        m.insert("parallel-worlds", vec!["parallel world", "alternate reality", "multiverse", "parallel universe"]);

        // Coming of Age & Identity
        m.insert("coming-of-age", vec!["coming of age", "growing up", "adolescence", "youth", "maturity"]);
        m.insert("identity", vec!["identity", "self-discovery", "finding oneself", "who am i"]);
        m.insert("lgbtq", vec!["lgbtq", "lgbt", "queer", "gay", "lesbian", "transgender", "bisexual"]);
        m.insert("race", vec!["race", "racism", "racial", "discrimination", "prejudice"]);
        m.insert("gender", vec!["gender", "feminism", "feminist", "patriarchy", "women's rights"]);

        // Social Issues
        m.insert("mental-health", vec!["mental health", "depression", "anxiety", "ptsd", "trauma", "therapy"]);
        m.insert("addiction", vec!["addiction", "alcoholism", "drug abuse", "substance abuse"]);
        m.insert("poverty", vec!["poverty", "poor", "homelessness", "inequality", "class struggle"]);
        m.insert("immigration", vec!["immigration", "immigrant", "refugee", "migration", "diaspora"]);
        m.insert("climate-change", vec!["climate change", "global warming", "environment", "ecological"]);

        // Historical Periods
        m.insert("victorian", vec!["victorian", "victorian era", "19th century", "1800s"]);
        m.insert("medieval", vec!["medieval", "middle ages", "dark ages", "knights", "castles"]);
        m.insert("renaissance", vec!["renaissance", "elizabethan", "tudor"]);
        m.insert("world-war", vec!["world war", "wwi", "wwii", "ww1", "ww2", "great war"]);
        m.insert("ancient", vec!["ancient", "antiquity", "classical", "roman", "greek"]);

        // Adventure & Quest
        m.insert("survival", vec!["survival", "survive", "surviving", "wilderness"]);
        m.insert("exploration", vec!["exploration", "explore", "discovery", "expedition", "adventure"]);
        m.insert("quest", vec!["quest", "journey", "pilgrimage", "odyssey"]);
        m.insert("heist", vec!["heist", "robbery", "theft", "con", "caper"]);

        // Supernatural & Paranormal
        m.insert("vampire", vec!["vampire", "vampires", "bloodsucker", "undead"]);
        m.insert("werewolf", vec!["werewolf", "werewolves", "lycanthrope", "shapeshifter"]);
        m.insert("ghost", vec!["ghost", "ghosts", "haunted", "haunting", "spirit", "specter"]);
        m.insert("demon", vec!["demon", "demons", "devil", "demonic", "hell"]);
        m.insert("angel", vec!["angel", "angels", "angelic", "heaven", "divine"]);

        // Mystery & Crime
        m.insert("detective", vec!["detective", "investigation", "investigator", "sleuth", "private eye"]);
        m.insert("serial-killer", vec!["serial killer", "psychopath", "murderer"]);
        m.insert("conspiracy", vec!["conspiracy", "cover-up", "secret society", "illuminati"]);

        // Character Types
        m.insert("female-protagonist", vec!["female lead", "female protagonist", "strong woman", "heroine", "female character"]);
        m.insert("male-protagonist", vec!["male lead", "male protagonist", "hero", "male character"]);
        m.insert("anti-hero", vec!["anti-hero", "antihero", "morally gray", "morally ambiguous"]);
        m.insert("chosen-one", vec!["chosen one", "prophecy", "destined", "savior"]);

        // Religion & Philosophy
        m.insert("religion", vec!["religion", "religious", "faith", "spiritual", "god", "deity"]);
        m.insert("atheism", vec!["atheism", "atheist", "secular", "non-believer"]);
        m.insert("existentialism", vec!["existential", "existentialism", "meaning of life", "absurdism"]);

        m
    };

    /// Historical periods and eras
    pub static ref HISTORICAL_PERIODS: HashMap<&'static str, (i32, i32)> = {
        let mut m = HashMap::new();
        m.insert("ancient", (0, 500));
        m.insert("medieval", (500, 1500));
        m.insert("renaissance", (1400, 1600));
        m.insert("victorian", (1837, 1901));
        m.insert("edwardian", (1901, 1910));
        m.insert("world war i", (1914, 1918));
        m.insert("world war ii", (1939, 1945));
        m.insert("cold war", (1947, 1991));
        m.insert("modern", (1950, 2000));
        m.insert("contemporary", (2000, 2030));
        m
    };

    /// Common stop words to ignore
    pub static ref STOP_WORDS: Vec<&'static str> = vec![
        "the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for",
        "of", "with", "by", "from", "about", "as", "into", "through", "during",
        "please", "recommend", "suggest", "find", "looking", "want", "need",
        "book", "books", "novel", "novels", "read", "reading", "good", "great", "best",
        "can", "you", "give", "me", "some", "any", "show"
    ];
}

impl EnhancedQuery {
    /// Create a new enhanced query from user input
    pub fn from_query(query: &str) -> Self {
        let query_lower = query.to_lowercase();
        let mut pattern = QueryPattern::General;
        let mut extracted_terms = Vec::new();
        let mut expanded_terms = Vec::new();
        let mut filters = QueryFilters::default();
        let mut hints = SearchHints::default();

        // Check for author queries (highest priority)
        for pattern_regex in AUTHOR_PATTERNS.iter() {
            if let Some(captures) = pattern_regex.captures(query) {
                if let Some(author_match) = captures.get(1) {
                    let author = author_match.as_str().trim().to_string();
                    if !author.is_empty() && author.len() > 2 {
                        pattern = QueryPattern::Author;
                        extracted_terms.push(author.clone());
                        filters.author = Some(author);
                        hints.metadata_weight = 0.8;
                        hints.semantic_weight = 0.2;
                        break;
                    }
                }
            }
        }

        // Check for genre queries
        if pattern == QueryPattern::General {
            for pattern_regex in GENRE_PATTERNS.iter() {
                if let Some(captures) = pattern_regex.captures(&query_lower) {
                    if let Some(genre_match) = captures.get(1) {
                        let genre = genre_match.as_str().trim();
                        for (base_genre, expansions) in GENRE_EXPANSIONS.iter() {
                            if expansions
                                .iter()
                                .any(|&exp| genre.contains(exp) || exp.contains(genre))
                            {
                                pattern = QueryPattern::Genre;
                                extracted_terms.push(base_genre.to_string());
                                filters.genres =
                                    expansions.iter().map(|&s| s.to_string()).collect();
                                expanded_terms.extend(expansions.iter().map(|&s| s.to_string()));
                                hints.semantic_weight = 0.7;
                                hints.metadata_weight = 0.3;
                                break;
                            }
                        }
                        if pattern == QueryPattern::Genre {
                            break;
                        }
                    }
                }
            }

            // Also check for genre keywords in general text
            if pattern == QueryPattern::General {
                for (base_genre, expansions) in GENRE_EXPANSIONS.iter() {
                    if expansions.iter().any(|&exp| query_lower.contains(exp)) {
                        pattern = QueryPattern::Genre;
                        extracted_terms.push(base_genre.to_string());
                        filters.genres = expansions.iter().map(|&s| s.to_string()).collect();
                        expanded_terms.extend(expansions.iter().map(|&s| s.to_string()));
                        hints.semantic_weight = 0.7;
                        hints.metadata_weight = 0.3;
                        break;
                    }
                }
            }
        }

        // Check for setting patterns
        for setting_pattern in SETTING_PATTERNS.iter() {
            if let Some(captures) = setting_pattern.captures(&query_lower) {
                if let Some(setting_match) = captures.get(1) {
                    let setting = setting_match.as_str().trim().to_string();
                    filters.settings.push(setting.clone());
                    extracted_terms.push(setting);
                    if pattern == QueryPattern::General {
                        pattern = QueryPattern::Setting;
                    }
                    break;
                }
            }
        }

        // Check for mood patterns
        if MOOD_PATTERNS.iter().any(|p| p.is_match(&query_lower))
            && pattern == QueryPattern::General
        {
            pattern = QueryPattern::Mood;
            hints.semantic_weight = 0.8;
        }

        // Check for pace patterns
        if PACE_PATTERNS.iter().any(|p| p.is_match(&query_lower))
            && pattern == QueryPattern::General
        {
            pattern = QueryPattern::Pace;
            hints.semantic_weight = 0.7;
        }

        // Check for perspective patterns
        if PERSPECTIVE_PATTERNS
            .iter()
            .any(|p| p.is_match(&query_lower))
            && pattern == QueryPattern::General
        {
            pattern = QueryPattern::Perspective;
            hints.semantic_weight = 0.6;
        }

        // Check for similar-to patterns
        if SIMILAR_PATTERNS.iter().any(|p| p.is_match(&query_lower)) {
            pattern = QueryPattern::SimilarTo;
            hints.semantic_weight = 0.9;
        }

        // Check for time-based queries
        if TIME_PATTERNS.iter().any(|p| p.is_match(&query_lower)) {
            if query_lower.contains("recent")
                || query_lower.contains("new")
                || query_lower.contains("modern")
                || query_lower.contains("contemporary")
            {
                filters.min_year = Some(2015);
                hints.recency_boost = 1.3;
            } else if query_lower.contains("classic") || query_lower.contains("old") {
                filters.max_year = Some(2000);
                hints.rating_boost = 1.2;
            }
            if pattern == QueryPattern::General {
                pattern = QueryPattern::TimeBased;
            }
        }

        // Check for historical periods
        for (period, (start_year, end_year)) in HISTORICAL_PERIODS.iter() {
            if query_lower.contains(period) {
                filters.min_year = Some(*start_year);
                filters.max_year = Some(*end_year);
                filters.settings.push(period.to_string());
                if pattern == QueryPattern::General {
                    pattern = QueryPattern::Setting;
                }
            }
        }

        // Check for audience patterns
        for audience_pattern in AUDIENCE_PATTERNS.iter() {
            if audience_pattern.is_match(&query_lower) {
                if query_lower.contains("kid") || query_lower.contains("child") {
                    filters.audience = Some("children".to_string());
                } else if query_lower.contains("teen")
                    || query_lower.contains("ya")
                    || query_lower.contains("young adult")
                {
                    filters.audience = Some("young adult".to_string());
                }
                if pattern == QueryPattern::General {
                    pattern = QueryPattern::Audience;
                }
                break;
            }
        }

        // Check for length patterns
        if LENGTH_PATTERNS.iter().any(|p| p.is_match(&query_lower)) {
            if query_lower.contains("short")
                || query_lower.contains("quick")
                || query_lower.contains("brief")
            {
                filters.max_pages = Some(300);
            }
            if pattern == QueryPattern::General {
                pattern = QueryPattern::Length;
            }
        }

        // Check for complexity patterns
        if COMPLEXITY_PATTERNS.iter().any(|p| p.is_match(&query_lower)) {
            if query_lower.contains("easy")
                || query_lower.contains("simple")
                || query_lower.contains("accessible")
            {
                hints.rating_boost = 1.1;
            }
            if pattern == QueryPattern::General {
                pattern = QueryPattern::Complexity;
            }
        }

        // Extract theme keywords with word boundary matching
        for (theme, keywords) in THEME_KEYWORDS.iter() {
            let has_match = keywords.iter().any(|&kw| {
                // Split multi-word keywords
                if kw.contains(' ') {
                    // For multi-word keywords, check if they appear as a phrase
                    query_lower.contains(kw)
                } else {
                    // For single-word keywords, use word boundaries
                    let words: Vec<&str> = query_lower.split_whitespace().collect();
                    words.iter().any(|&word| {
                        // Remove punctuation and check for exact match
                        let clean_word = word.trim_matches(|c: char| !c.is_alphanumeric());
                        clean_word == kw
                    })
                }
            });

            if has_match {
                extracted_terms.push(theme.to_string());
                expanded_terms.extend(keywords.iter().map(|&s| s.to_string()));
                filters.themes.push(theme.to_string());
                if pattern == QueryPattern::General {
                    pattern = QueryPattern::Theme;
                }
            }
        }

        // Extract general meaningful terms (non-stop words)
        for word in query_lower.split_whitespace() {
            let clean_word = word.trim_matches(|c: char| !c.is_alphanumeric());
            if clean_word.len() > 3
                && !STOP_WORDS.contains(&clean_word)
                && !extracted_terms.contains(&clean_word.to_string())
            {
                extracted_terms.push(clean_word.to_string());
            }
        }

        // Set minimum rating filter for quality queries
        if query_lower.contains("best")
            || query_lower.contains("top")
            || query_lower.contains("highly rated")
        {
            filters.min_rating = Some(4.0);
            hints.rating_boost = 1.5;
        }

        Self {
            original_query: query.to_string(),
            pattern,
            extracted_terms,
            expanded_terms,
            filters,
            search_hints: hints,
        }
    }
}
