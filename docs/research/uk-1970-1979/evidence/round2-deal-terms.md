## Round 2: Producer points and publishing advances

### Verdicts

- **Producer: UNRESOLVED.** No 1970s UK producer rate with a named
  producer, artist, year and a retail-vs-royalty basis was located in
  the sources reachable this round. The research problem is not a
  shortage of producers who took points; it is that the rate lived in
  private contracts and in trade-press deal reports that this archive
  cannot be searched reliably (see "Method failure" below). **Do not put
  a producer points number into the game from this round.**
- **Publishing: PARTIALLY RESOLVED.** The *structure* of the 1970s UK
  songwriter deal is now documented from two independent sources with a
  specific mechanism the first pass did not have: a **double 50/50
  split**, which leaves the writer 25% in the US, not 50%. What is still
  missing is a **mid-range advance figure** — the first-pass £50
  (Schroeder v Macaulay 1974) remains the only litigated number and is a
  floor, not a norm.

### New findings

| Year(s) | Location | Category | Finding | Amount | Unit / coverage | Includes | Source URL | Source type | Evidence class | Confidence | Proposed gameplay use | Notes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1967 deal; described 2020s | UK / US | Publishing — split | Elton John's Dick James (DJM) contract split royalties **50/50 in the UK, then 50/50 again between DJM's UK and US companies**, leaving the writer **25%** of US income. Napier-Bell: "in the USA Dick James got a total of 75% of each song and Elton only 25%… In the 1970s, this sort of set-up was quite normal." | 50/50 UK; 25% writer in US | Share of song revenue, by territory | Both publishing and writer shares | https://simonnapierbell.substack.com/p/dick-james-and-the-joy-of-publishing | Practitioner memoir (ex-manager) | Anecdote (informed, first-hand industry) | High for the mechanism; medium that it was universal | Model publishing income as **territory-dependent**: writer keeps ~50% domestically, ~25% on US income under a sub-publishing deal. This is a real, checkable 1970s trap for the player. | Confirms and sharpens the established "50:50" fact — the second split is the part the first pass missed. Non-UK leg flagged: the 25% applies to US income. |
| 1970s (stated 1977) | UK | Publishing — mechanical rate | Music Week reports the UK **mechanical royalty at 6¼ per cent** of retail ("the 6¼ percent mechanical payment"), and gives worked per-unit figures showing how it divides among tracks. | 6.25% of retail | Statutory-style mechanical rate, per record sold | Publisher's gross mechanical | https://www.worldradiohistory.com/UK/Music-Week/1977/Music-Week-1977-07-16.pdf | Trade press (scanned, OCR) | Documented fact | High | Gives the game a hard, citable number for the *publishing* income pool per unit — the denominator an advance is recouped against. | Quote is from the Terry Gates / Eaton Music piece on unequal division of mechanicals between album sides. Per-work examples: 1.73p, 0.943p, 1.85p vs 0.925p per copy. |
| 1977 | UK | Publishing — publisher economics | Same piece: a publisher's take on a hit album side was on the order of **£900–£1,500** for a writer's share, and the complaint is that track placement on an album shifts a writer's income by "almost 100 percent". | ~£900 vs ~£1,500 | Writer income per album side (worked example) | Writer's share, one album | https://www.worldradiohistory.com/UK/Music-Week/1977/Music-Week-1977-07-16.pdf | Trade press (scanned, OCR) | Estimate (journalist's worked example, not a contract) | Medium | Scale check: publishing income on a mid-success album is **hundreds, not thousands, of pounds** — so a £50–£500 advance was 1–6 months of earnings, i.e. genuinely exploitative. | Numbers are the article's own arithmetic under "current split terms", not a named deal. |
| 1974 (HL) | UK | Publishing — advance floor | Established, restated only as anchor: **£50 total advance**, 5-year form, auto-extending above £5,000 royalties → voided in *Schroeder v Macaulay*. | £50 | Total advance, standard form | — | (first pass) | Case law | Documented fact | High | Already in game. Keep as the **worst-case** deal, not the average. | Not re-verified this round per instructions. |

### Method failure (record this — it saves a future round)

The Music Week scan archive **is** real and rich, but is not
phrase-searchable the way the brief assumed:

- URL pattern is **not** `.../YYYY/Music-Week-YYYY-MM-DD.pdf` for all
  years. Verified working list comes from the index at
  `https://www.worldradiohistory.com/Music_Week.htm` (159 issues for the
  1970s). Directory listings (`/UK/Music-Week/`) return **403**.
- Of 40 issues downloaded (every 4th), **only 14 had a text layer**; the
  rest are image-only (1978–79 almost entirely). `pdftotext` yields
  nothing.
- Where a text layer exists the OCR is **multi-column and interleaved**,
  so a phrase like "producer royalty" or "publishing advance" is broken
  across lines. Searching `"per cent of retail"`, `"producer royalty"`,
  `"publishing advance"`, `"points on"` produced **zero** genuine hits
  across all 14 issues. Hits that looked promising were column-bleed
  false positives.
- **Consequence:** this archive is usable only by *browsing specific
  pages as images*, or after OCR with a modern engine
  (`ocrmypdf`/tesseract) that handles columns. That is the single
  highest-value next step and it is cheap.

Other dead ends this round, so they are not retried blindly: Google
Books API returns **429 quota exceeded** for all queries; `archive.org`
full-text endpoints (`ia-fts.../hits`, `services/search/v1/scrape`)
returned empty/`total 0`; Billboard archive index paths return 403/404;
Sound On Sound's on-site search is JS-driven and returns only nav links
to `curl`.

### What this changes for the game

- **Producer points must stay unmodelled or explicitly flagged as a
  placeholder.** The honest 1970s state of knowledge is "independent
  producers took a share, basis varied between retail and royalty, size
  unknown". If the game must ship a number, it should be labelled
  in-game as an estimate and kept well below the modern 2–4 points norm
  — a modern figure would silently import 1990s practice into a 1970s
  economy.
- **Model publishing as two separate splits, not one.** The player
  should experience the UK 50/50 and then discover that US income
  arrives at 25% because of a sub-publishing split. That is a
  documentable, era-correct mechanic and it is more interesting than a
  flat rate.
- **Give the player the mechanical pool as a visible denominator.** At
  6.25% of retail in 1977, with per-work fractions of a penny, the game
  can show *why* an advance is hard to recoup and why publishers pushed
  for co-writing credit — this makes the advance decision legible
  instead of abstract.
- **Keep the £50 deal as a cautionary archetype, not the baseline.**
  Present it as the deal a desperate writer signs; the mid-range deal is
  still an open research question, so the game should not claim to know
  it.
- **Territory is a lever.** A publisher with a US affiliate is worth
  more to the writer than one without, and the game can price that
  difference.

### Still unresolved

- **Producer rate, 1970s UK, with numbers.** No named producer + artist
  + year + rate + basis found. To settle:
- OCR the 159-issue Music Week set properly (`ocrmypdf --deskew
  --rotate-pages`), then grep for `"points"`, `"production deal"`,
  `"producer"` near `"per cent"`. Deal reports and the "studio news"
  columns are where these appear.
- **Melody Maker / NME / Sounds** 1970s runs — same treatment; producer
  interviews there are more candid than Music Week's.
- **Court records**: Kinks/Talmy and Who/Talmy litigation files (Talmy's
  own contract terms are the one independent-producer rate known to
  exist in a legal record). Also *Bolan v. ...* and the T. Rex
  management disputes.
- Producer autobiographies that state terms: Visconti, *Bowie, Bolan and
  the Brooklyn Boy*; Ken Scott, *Abbey Road to Ziggy Stardust*; Alan
  Parsons, *Art & Science of Sound Recording*. These are
  borrowable/cheap and each may carry one rate.
- **Mid-range publishing advance, 1970s UK.** To settle:
- **Music Week "publishing" deal columns** (same OCR job) — small
  signings were routinely reported with advances.
- **Schroeder v Macaulay** case file and the surrounding 1970s
  litigation over standard-form publishing contracts: the pleadings cite
  other real advances as comparators.
- **Simon Napier-Bell, *Black Vinyl White Powder*** and *You Don't Have
  to Say You Love Me* — he names figures.
- **UK music publishing company archives**: Chappell/Essex/Carlin
  deposited business records; the **National Archives** business-records
  browse (discovery.nationalarchives.gov.uk, business records) surfaced
  in search and may hold publisher ledgers.
- **Whether producer points were ever paid on *retail* in the UK
  specifically**, or only on royalties. No evidence either way this
  round; the brief asserts both existed, but that assertion is currently
  unsourced.
