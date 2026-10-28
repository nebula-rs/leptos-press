//! Consulta sobre o [`SearchIndex`] e trecho com destaque. Puro: sem DOM e sem Leptos, testado
//! no host.
//!
//! - **Termos:** o texto da busca, normalizado (AICI, `leptos_nebula::search::normalize`) e
//!   separado por espaço; **todos** são obrigatórios (cada um precisa aparecer em algum campo da
//!   seção: título da página, cabeçalhos, texto ou código).
//! - **Ranking:** cada termo vale pelo melhor campo em que aparece — título da página (4) >
//!   cabeçalho (3) > texto (2) > código (1) —, e dentro do campo, início de palavra antes de meio
//!   de palavra (`campo × 2 + 1` contra `campo × 2`). A seção soma os termos. Empate: a ordem do
//!   site (a seção do topo antes das outras da mesma página). No máximo [`MAX_RESULTS`].
//! - **Trecho:** ~[`SNIPPET_CHARS`] caracteres do texto **original** em volta do primeiro termo
//!   achado, com os termos marcados — o mapa de posições vem de
//!   `leptos_nebula::search::normalize_with_map`.

use leptos_nebula::search::{normalize, normalize_with_map};

use super::index::{SearchIndex, Section};

/// Resultados no máximo.
pub const MAX_RESULTS: usize = 50;

/// Tamanho aproximado do trecho, em caracteres.
pub const SNIPPET_CHARS: usize = 120;

/// Uma seção achada.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hit {
    /// Índice em [`SearchIndex::sections`].
    pub section: usize,
    pub score: u32,
}

/// Resultados de uma página, na ordem de exibição.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HitGroup {
    /// Índice em [`SearchIndex::pages`].
    pub page: usize,
    /// Seções (índices em [`SearchIndex::sections`]), da mais relevante para a menos.
    pub sections: Vec<usize>,
}

/// Pedaço de texto; `mark` = é um termo achado (vai num `<mark>`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    pub text: String,
    pub mark: bool,
}

/// Termos normalizados da busca: separados por espaço, sem repetidos, na ordem digitada.
pub fn terms(query: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for t in normalize(query).split_whitespace() {
        if !out.iter().any(|o| o == t) {
            out.push(t.to_string());
        }
    }
    out
}

/// Seções que contêm **todos** os termos, por relevância (no máximo [`MAX_RESULTS`]).
/// Sem termos, nada.
pub fn search(index: &SearchIndex, terms: &[String]) -> Vec<Hit> {
    if terms.is_empty() {
        return Vec::new();
    }
    let mut hits: Vec<Hit> = index
        .sections
        .iter()
        .enumerate()
        .filter_map(|(i, s)| {
            let title = &index.pages[s.page].norm_title;
            let mut score = 0;
            for t in terms {
                score += term_score(title, s, t)?;
            }
            Some(Hit { section: i, score })
        })
        .collect();
    // Estável: no empate, a ordem do site.
    hits.sort_by_key(|h| std::cmp::Reverse(h.score));
    hits.truncate(MAX_RESULTS);
    hits
}

/// Agrupa por página, na ordem do primeiro resultado de cada uma.
pub fn group(index: &SearchIndex, hits: &[Hit]) -> Vec<HitGroup> {
    let mut groups: Vec<HitGroup> = Vec::new();
    for hit in hits {
        let page = index.sections[hit.section].page;
        match groups.iter_mut().find(|g| g.page == page) {
            Some(g) => g.sections.push(hit.section),
            None => groups.push(HitGroup {
                page,
                sections: vec![hit.section],
            }),
        }
    }
    groups
}

/// Valor do termo na seção: o melhor campo × 2, + 1 se começa uma palavra; `None` se o termo
/// não aparece em nenhum campo.
fn term_score(title: &str, s: &Section, term: &str) -> Option<u32> {
    let fields: [(&str, u32); 4] = [
        (title, 4),
        (&s.norm_headings, 3),
        (&s.norm_text, 2),
        (&s.norm_code, 1),
    ];
    fields.iter().find_map(|(field, weight)| {
        let mut found = false;
        for (at, _) in field.match_indices(term) {
            if word_start(field, at) {
                return Some(weight * 2 + 1);
            }
            found = true;
        }
        found.then_some(weight * 2)
    })
}

/// A posição `at` começa uma palavra (início do texto ou depois de algo que não é letra/dígito).
fn word_start(text: &str, at: usize) -> bool {
    text[..at]
        .chars()
        .next_back()
        .is_none_or(|c| !c.is_alphanumeric())
}

/// O texto original inteiro, com os termos marcados (para o caminho de cabeçalhos).
pub fn highlight(original: &str, terms: &[String]) -> Vec<Segment> {
    let (norm, map) = normalize_with_map(original);
    let ranges = marks(original, &norm, &map, terms);
    segments(original, 0, original.len(), &ranges)
}

/// Trecho do texto original em volta do primeiro termo achado (o mais à esquerda), com uns
/// [`SNIPPET_CHARS`] caracteres e os termos marcados; `…` nas pontas cortadas. Sem termo no
/// texto, o começo do texto, sem marcas.
pub fn snippet(original: &str, terms: &[String]) -> Vec<Segment> {
    let (norm, map) = normalize_with_map(original);
    let first = terms
        .iter()
        .filter(|t| !t.is_empty())
        .filter_map(|t| norm.find(t.as_str()))
        .min();
    let anchor = first.map_or(0, |p| map[p]);
    // Um terço do trecho antes do termo, o resto depois.
    let before = SNIPPET_CHARS / 3;
    let mut start = anchor;
    for _ in 0..before {
        match original[..start].chars().next_back() {
            Some(c) => start -= c.len_utf8(),
            None => break,
        }
    }
    // Começa numa palavra inteira, se houver espaço por perto.
    if start > 0
        && let Some(sp) = original[start..anchor].find(' ')
    {
        start += sp + 1;
    }
    let mut end = start;
    for c in original[start..].chars().take(SNIPPET_CHARS) {
        end += c.len_utf8();
    }
    // Termina numa palavra inteira (sem cortar o próprio termo).
    if end < original.len()
        && let Some(sp) = original[anchor..end].rfind(' ')
        && sp > 0
    {
        end = anchor + sp;
    }
    let ranges = marks(original, &norm, &map, terms);
    let mut out = Vec::new();
    if start > 0 {
        out.push(plain("…"));
    }
    out.extend(segments(original, start, end, &ranges));
    if end < original.len() {
        out.push(plain("…"));
    }
    out
}

fn plain(text: &str) -> Segment {
    Segment {
        text: text.to_string(),
        mark: false,
    }
}

/// Trechos (no original) de todas as ocorrências dos termos, ordenados e unidos.
fn marks(original: &str, norm: &str, map: &[usize], terms: &[String]) -> Vec<(usize, usize)> {
    let mut ranges: Vec<(usize, usize)> = Vec::new();
    for t in terms.iter().filter(|t| !t.is_empty()) {
        for (s, m) in norm.match_indices(t.as_str()) {
            ranges.push(original_range(map, s, s + m.len(), original.len()));
        }
    }
    ranges.sort_unstable();
    let mut merged: Vec<(usize, usize)> = Vec::new();
    for (s, e) in ranges {
        match merged.last_mut() {
            Some(last) if s <= last.1 => last.1 = last.1.max(e),
            _ => merged.push((s, e)),
        }
    }
    merged
}

/// Trecho `[s, e)` do normalizado → trecho do original (regra de `normalize_with_map`).
fn original_range(map: &[usize], s: usize, e: usize, len: usize) -> (usize, usize) {
    let last = map[e - 1];
    let end = map[e..].iter().copied().find(|&o| o > last).unwrap_or(len);
    (map[s], end)
}

/// `original[from..to]` em pedaços, marcando o que cai nos `ranges`.
fn segments(original: &str, from: usize, to: usize, ranges: &[(usize, usize)]) -> Vec<Segment> {
    let mut out = Vec::new();
    let mut at = from;
    for &(s, e) in ranges {
        let (s, e) = (s.max(from), e.min(to));
        if s >= e {
            continue;
        }
        if s > at {
            out.push(plain(&original[at..s]));
        }
        out.push(Segment {
            text: original[s..e].to_string(),
            mark: true,
        });
        at = e;
    }
    if at < to {
        out.push(plain(&original[at..to]));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::index::tests::{FULL, SITE};

    fn index() -> SearchIndex {
        SearchIndex::build(FULL, &SITE)
    }

    fn found(index: &SearchIndex, q: &str) -> Vec<String> {
        search(index, &terms(q))
            .iter()
            .map(|h| index.href(h.section))
            .collect()
    }

    fn render(segments: &[Segment]) -> String {
        segments
            .iter()
            .map(|s| {
                if s.mark {
                    format!("[{}]", s.text)
                } else {
                    s.text.clone()
                }
            })
            .collect()
    }

    #[test]
    fn terms_are_normalized_split_and_deduplicated() {
        assert_eq!(terms("  São  PAULO são "), ["sao", "paulo"]);
        assert!(terms("   ").is_empty());
        assert!(search(&index(), &terms("")).is_empty());
    }

    #[test]
    fn every_term_is_required() {
        let index = index();
        assert_eq!(
            found(&index, "key_as_value"),
            ["/components/select#formato-3-com-key_as_value"]
        );
        // "modelo" está em duas seções; com "key" só sobra uma
        assert_eq!(
            found(&index, "modelo"),
            [
                "/components/select#formatos-de-options",
                "/components/select#formato-3-com-key_as_value"
            ]
        );
        assert_eq!(
            found(&index, "modelo key"),
            ["/components/select#formato-3-com-key_as_value"]
        );
        assert!(found(&index, "modelo inexistente").is_empty());
    }

    #[test]
    fn accent_and_case_insensitive() {
        let index = index();
        assert_eq!(found(&index, "sao"), ["/components/select"]);
        assert_eq!(found(&index, "SÃO paulo"), ["/components/select"]);
        assert_eq!(found(&index, "instalacao"), ["/guide/start#instalação"]);
    }

    #[test]
    fn title_beats_heading_beats_text_beats_code() {
        let index = index();
        // "select" só está no título da página: todas as seções dela, topo primeiro
        let all = found(&index, "select");
        assert_eq!(all.first().map(String::as_str), Some("/components/select"));
        assert_eq!(all.len(), 9);
        // "filtro": cabeçalho (Filtro) > texto (API: "liga o filtro")
        assert_eq!(
            found(&index, "filtro"),
            ["/components/select#filtro", "/components/select#api"]
        );
        // "topo": texto da seção do topo de Começando e de Select; empate pela ordem do site
        assert_eq!(
            found(&index, "topo"),
            ["/guide/start", "/components/select"]
        );
        // "ObjectOptions" só está no código
        let hits = search(&index, &terms("objectoptions"));
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].score, 3);
        // cabeçalho ganha de texto e código: "key" está no cabeçalho, no texto e no código
        let hits = search(&index, &terms("key"));
        let best = &index.sections[hits[0].section];
        assert_eq!(best.anchor, Some("formato-3-com-key_as_value"));
    }

    #[test]
    fn word_start_beats_middle_of_word() {
        let index = index();
        // no texto do topo de Select, "ao" só aparece no meio ("opção", "São"); "sa", no início
        let mid = search(&index, &terms("ao"));
        let start = search(&index, &terms("sa"));
        let top = index.sections.iter().position(|s| s.page == 3).unwrap();
        let score = |hits: &[Hit]| hits.iter().find(|h| h.section == top).map(|h| h.score);
        assert_eq!(score(&mid), Some(4)); // texto, meio de palavra
        assert_eq!(score(&start), Some(5)); // texto, início de palavra
        assert!(word_start("a_b", 2));
        assert!(word_start("x", 0));
        assert!(!word_start("ab", 1));
    }

    #[test]
    fn at_most_fifty_results() {
        let mut full = String::new();
        for _ in 0..60 {
            full.push_str("## Igual\n\ntexto\n\n");
        }
        let full = format!("---\nurl: /guide.md\n---\n\n# Guia\n\n{full}");
        let index = SearchIndex::build(&full, &SITE);
        assert_eq!(index.sections.len(), 61);
        assert_eq!(search(&index, &terms("texto")).len(), MAX_RESULTS);
    }

    #[test]
    fn groups_follow_the_first_hit_of_each_page() {
        let index = index();
        let hits = search(&index, &terms("topo"));
        let groups = group(&index, &hits);
        assert_eq!(groups.iter().map(|g| g.page).collect::<Vec<_>>(), [2, 3]);
        let hits = search(&index, &terms("o"));
        let groups = group(&index, &hits);
        let flat: usize = groups.iter().map(|g| g.sections.len()).sum();
        assert_eq!(flat, hits.len());
        let mut pages: Vec<usize> = groups.iter().map(|g| g.page).collect();
        pages.dedup();
        assert_eq!(pages.len(), groups.len());
    }

    #[test]
    fn snippet_marks_terms_in_the_original_text() {
        let t = terms("sao paulo");
        assert_eq!(
            render(&snippet("Escolha uma opção. São Paulo fica no topo.", &t)),
            "Escolha uma opção. [São] [Paulo] fica no topo."
        );
        assert_eq!(
            render(&highlight("Formatos de opções", &terms("opcoes"))),
            "Formatos de [opções]"
        );
        // termos sobrepostos viram uma marca só
        assert_eq!(render(&highlight("Ação", &terms("aca cao"))), "[Ação]");
        // sem termo no texto: o começo, sem marcas
        assert_eq!(render(&snippet("Nada aqui.", &t)), "Nada aqui.");
    }

    #[test]
    fn snippet_cuts_around_the_first_term() {
        let long = format!(
            "{} No meio está o Alvo procurado. {}",
            "antes ".repeat(40),
            "depois ".repeat(40)
        );
        let seg = snippet(&long, &terms("alvo"));
        let text = render(&seg);
        assert!(text.starts_with('…') && text.ends_with('…'), "{text}");
        assert!(text.contains("[Alvo]"), "{text}");
        let chars = text.chars().count();
        assert!((80..=130).contains(&chars), "{chars}: {text}");
        // corta em palavra inteira
        assert!(
            text.starts_with("…antes ") || text.starts_with("…No"),
            "{text}"
        );
        assert!(
            !text.contains("antes antes antes antes antes antes antes antes antes antes antes")
        );
    }

    #[test]
    fn snippet_with_letters_that_change_size() {
        // ẞ (3 bytes) → ß (2 bytes): a marca cobre a letra original inteira
        assert_eq!(
            render(&highlight("GROẞE Straße", &terms("große"))),
            "[GROẞE] Straße"
        );
        assert_eq!(render(&highlight("İzmir", &terms("izm"))), "[İzm]ir");
    }
}
