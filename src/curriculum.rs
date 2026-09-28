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
    #[serde(rename = "week")]
    pub _week: String,
    pub module: u8,
    pub question: String,
    pub answers: Vec<String>,
    pub correct: usize,
    pub explain: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Achievement {
    pub id: String,
    pub name: String,
    pub desc: String,
    pub xp: u32,
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
    #[allow(dead_code)]
    pub sha256: String,
    #[allow(dead_code)]
    pub file: String,
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
    #[allow(dead_code)]
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
    pub fn load() -> Curriculum {
        let raw = include_str!("../assets/curriculum.json");
        let mut c: Curriculum = serde_json::from_str(raw).expect("curriculum.json is invalid");
        c.drills = serde_json::from_str(include_str!("../assets/drills.json"))
            .expect("drills.json is invalid");
        c
    }

    pub fn module_name(&self, id: u8) -> String {
        self.modules
            .iter()
            .find(|m| m.id == id)
            .map(|m| format!("{} {}", m.icon, m.name))
            .unwrap_or_else(|| "—".to_string())
    }

}
