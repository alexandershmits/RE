use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Course {
    pub title: String,
    pub subtitle: String,
    pub rules: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Module {
    pub id: u8,
    pub name: String,
    pub icon: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Lab {
    pub title: String,
    pub steps: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Week {
    pub id: String,
    pub num: u32,
    pub num_end: u32,
    pub title: String,
    pub module: u8,
    pub lectures: Vec<String>,
    pub lab: Option<Lab>,
    pub psets: Vec<String>,
    pub checkpoint: Vec<String>,
    #[serde(default)]
    pub case: Option<String>,
}

impl Week {
    pub fn label(&self) -> String {
        if self.num == self.num_end {
            format!("Неделя {}", self.num)
        } else {
            format!("Недели {}–{}", self.num, self.num_end)
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Quiz {
    pub id: String,
    pub week: String,
    pub module: u8,
    pub question: String,
    pub answers: Vec<String>,
    pub correct: usize,
    pub explain: String,
}

/// Условие получения ачивки. Все заданные пункты должны выполняться одновременно.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Rule {
    /// Эти недели отмечены закрытыми.
    pub weeks: Vec<String>,
    /// У этих недель сданы все PSet.
    pub psets: Vec<String>,
    /// В этих модулях все квизы решены верно.
    pub quiz_modules: Vec<u8>,
    /// Не менее N% всех квизов решены верно.
    pub quiz_percent: Option<u32>,
    /// Решено не менее N встроенных челленджей.
    pub challenges: Option<u32>,
}

impl Rule {
    pub fn is_empty(&self) -> bool {
        self.weeks.is_empty()
            && self.psets.is_empty()
            && self.quiz_modules.is_empty()
            && self.quiz_percent.is_none()
            && self.challenges.is_none()
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Achievement {
    pub id: String,
    pub name: String,
    pub desc: String,
    pub xp: u32,
    #[serde(default)]
    pub when: Rule,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Resource {
    pub name: String,
    pub url: String,
    pub category: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Flashcard {
    pub id: String,
    pub front: String,
    pub back: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct InterviewQuestion {
    pub q: String,
    pub a: String,
    pub cat: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PlacementQ {
    pub q: String,
    pub a: Vec<String>,
    pub correct: usize,
    pub week: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AsmDrill {
    pub c: String,
    pub asm: Vec<String>,
    pub answers: Vec<String>,
    pub correct: usize,
    pub explain: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AddrDrill {
    pub q: String,
    pub answers: Vec<String>,
    pub correct: usize,
    pub explain: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PatternDrill {
    pub asm: String,
    pub answers: Vec<String>,
    pub correct: usize,
    pub explain: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ScriptDrill {
    pub task: String,
    pub answer: String,
    pub hint: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Drills {
    #[serde(default)]
    pub asm: Vec<AsmDrill>,
    #[serde(default)]
    pub addr: Vec<AddrDrill>,
    #[serde(default)]
    pub pattern: Vec<PatternDrill>,
    #[serde(default)]
    pub script: Vec<ScriptDrill>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Challenge {
    pub id: String,
    pub level: u8,
    pub title: String,
    pub desc: String,
    pub hint: String,
    pub flag: String,
    /// Первые 16 hex-символов SHA-256 ELF-бинаря.
    pub sha256: String,
    /// Первые 16 hex-символов SHA-256 PE-бинаря.
    #[serde(default)]
    pub sha256_exe: String,
    pub file: String,
    #[serde(default)]
    pub file_exe: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Ethalon {
    pub pset: String,
    pub lesson: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Diagram {
    pub title: String,
    pub code: String,
    pub tag: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Curriculum {
    pub version: u32,
    pub course: Course,
    pub modules: Vec<Module>,
    pub weeks: Vec<Week>,
    pub quizzes: Vec<Quiz>,
    pub achievements: Vec<Achievement>,
    pub resources: Vec<Resource>,
    #[serde(default)]
    pub flashcards: Vec<Flashcard>,
    #[serde(default)]
    pub interview_questions: Vec<InterviewQuestion>,
    #[serde(default)]
    pub rubric: Vec<String>,
    #[serde(default)]
    pub placement: Vec<PlacementQ>,
    #[serde(default)]
    pub diagrams: Vec<Diagram>,
    #[serde(default)]
    pub drills: Drills,
    #[serde(default)]
    pub challenges: Vec<Challenge>,
    #[serde(default)]
    pub topic_map: String,
    #[serde(default)]
    pub ethalon_writeups: Vec<Ethalon>,
}

impl Curriculum {
    /// Встроенный контент; падает только при дефекте сборки — его ловит тест `curriculum_parses`.
    pub fn load() -> Curriculum {
        Self::try_load().expect("встроенный контент повреждён")
    }

    pub fn try_load() -> Result<Curriculum, String> {
        let mut c: Curriculum = serde_json::from_str(include_str!("../assets/curriculum.json"))
            .map_err(|e| format!("curriculum.json: {e}"))?;
        c.drills = serde_json::from_str(include_str!("../assets/drills.json"))
            .map_err(|e| format!("drills.json: {e}"))?;
        Ok(c)
    }

    pub fn module_name(&self, id: u8) -> String {
        self.modules
            .iter()
            .find(|m| m.id == id)
            .map(|m| format!("{} {}", m.icon, m.name))
            .unwrap_or_else(|| "—".to_string())
    }

    pub fn quiz_by_id(&self, id: &str) -> Option<&Quiz> {
        self.quizzes.iter().find(|q| q.id == id)
    }

    pub fn week_by_id(&self, id: &str) -> Option<&Week> {
        self.weeks.iter().find(|w| w.id == id)
    }
}
