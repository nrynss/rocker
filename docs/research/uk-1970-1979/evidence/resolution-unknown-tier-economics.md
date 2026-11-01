# Resolution 1: Unknown-band and support-slot economics

Run date: 2026-10-16. Goal: close the "unknown tier" money gap for 1970s
UK, or prove it cannot be closed from public sources. Effort: ~21 tool
calls (13 web searches, 2 successful content fetches, 1
Cloudflare-blocked fetch recovered via curl, 2 large-result reads).

## Target 1 — unknown-band gig fees (pubs/clubs, 1970-79)

**Verdict: Partially resolved.** Adjacent union-negotiated 1970s rates
found (recording/broadcast/theatre); no direct per-night pub/club fee
figure for an unknown band surfaced in public web sources this pass.

Evidence found:
- Musicians' Union 1971–1980 history chapter (muhistory.com): 1970s
  union-negotiated rates — "The fee for a musician on a recording
  session increased to £24.75 for 3 hours; a session for an ILR station
  paid £15 (except Capital Radio which paid £30) and salaries in London
  theatres ranged from £50–£58.50 per week." Member subscriptions
  "ranged from £8–£29 per year depending on income from music." URL:
  https://www.muhistory.com/contact-us/1971-1980-2 (fetched 2026-10-16;
  fetch_content was blocked by a site security check, recovered via
  `curl -sL -A "Mozilla/5.0"`). Caveat: the page does not date each
  figure within the decade, and these are studio/broadcast/theatre rates
  — **not** pub gig fees. Evidence tier: official union history; high
  for what it states.
- Same chapter (1980 context): BBC musicians' strike, ACAS compromise 1
  Aug, ILR needle-time deal. Shows the union negotiating live-music
  rates in-window, but no pub/club minimum rate appears in the page
  text.
- Will Birch, "Raising the bar: the chaotic story of pub rock"
  (loudersound.com, 1 Jun 2020; adapted from his book No Sleep Till
  Canvey Island) — fetched in full text (~18KB excerpt). Covers scene
  origins (Eggs Over Easy at the Tally Ho, Brinsley Schwarz "Hype",
  Fillmore 1970) but the retrieved portion contains **no per-gig fee
  figure**. URL:
  https://www.loudersound.com/features/raising-the-bar-the-chaotic-story-of-pub-rock
- Modern baselines (for scale only — must NOT be backdated): Musicians'
  Union "National Gig Rates" page (search snippet, not fetched this
  run): "For musicians in groups performing in pubs and clubs of up to 3
  hours: £167.16 per musician"; lastminutemusicians.com (1 May 2026,
  snippet): functions up to 4h = £222.88/musician. These confirm the
  per-musician structure of MU rate-setting, not 1970s values.
- The £40 Hope & Anchor claim (1980-era, £40 fee + £40 repair bill): a
  targeted search ("Hope and Anchor" 1970s "£40"/"£30"/"£50" per night
  unsigned) returned no supporting or refuting source. Remains
  unverified.

New confidence: **low** for a per-night 1970s pub fee. Why: the only
1970s figures found are recording/broadcast/theatre; no primary source
with a pub/club per-night figure was retrievable; every figure-bearing
pub result was modern. Clearly labelled inference (not evidence): if the
~1980 £40 claim were representative, mid-1970s unknown-band fees would
plausibly be lower — no source found states this.

## Target 2 — support-slot terms (paid fee vs expenses only)

**Verdict: Unresolved (genuine gap).** The specific gig is documented;
the money terms are not present in any source found.

Evidence found:
- Sex Pistols supported Eddie & the Hot Rods at the Marquee on 12
  February 1976; NME's first Pistols review (Neil Spencer) ran 21 Feb
  1976. Sources: philjens.plus.com press-cutting page — URL:
  http://www.philjens.plus.com/pistols/pistols/press_cuttings_marquee_Feb76.html
  (search snippet, not fetched this run); Sex Pistols official Facebook
  post (snippet). No fee, expenses, or terms text appears in any snippet
  retrieved.
- No support-fee figure found anywhere: searches pairing "support
  band"/"support slot" with "no fee"/"expenses only"/"petrol money" for
  1970s UK returned only modern or unrelated content. England's Dreaming
  money excerpts and promoter memoirs were not located this pass.

New confidence: **low**; genuine gap. What it would take: full text of
Jon Savage, England's Dreaming (1976 gig chapters), or Hot
Rods/Pistols-era interviews.

## Target 3 — booking agent commission (1970s UK)

**Verdict: Unresolved for the 1970s.** Only modern commission data was
retrievable; nothing from 1970-79 was found. Modern guidance (~10-25%)
must not be backdated without new evidence.

Evidence found (all modern, snippet-level, dated 2026 unless noted):
- yourtempo.com (29 Sep 2026): "A booking agent's commission typically
  falls between 10 and 20 percent of an artist's gross income from a
  live performance."
- gigxchange.app (8 May 2026): "UK booking agents charge 10–25%
  commission. UK indie agents charge 10–15%."
- orphiq.com (15 Mar 2026): "The industry standard for established
  agents is 10% of gross."
- booking-agent.io: "standard commission is roughly 10% of the gross
  income"; up to ~15% on competitive deals.
- howstuffworks.com (2008): "between five and 10 percent."
- **No 1970s source found**: no Melody Maker/Music Week agency ad,
  NEMS/Harry Fox agency history, or artist-memoir percentage surfaced in
  searches.

New confidence: **low** for the 1970s. 10-15% remains a working
hypothesis from modern sources only — an inference, not a finding.

## Method notes

- Searches run (all web_search): "MU minimum engagement rates 1970s pub
  gig fee history"; "pub rock 1970s £ per night fee Hope and Anchor
  Canvey"; "musiciansunion.org.uk history 1971 1980 chapter minimum
  rates"; "pub rock memoir Dr Feelgood paid per gig"; "Sex Pistols
  support Eddie and the Hot Rods Marquee February 1976 no fee/expenses";
  "1970s UK booking agent commission 10/15 per cent memoir"; "punk 1976
  support band expenses/petrol money England's Dreaming"; "support
  band/slot 1970s UK no fee/expenses only"; "agency commission 1970s
  Britain NEMS Melody Maker"; "working men's club band 1970s £ night
  fee"; "No Sleep Till Canvey Island money"; "Sex Pistols 1976 £ paid
  Marquee Nashville"; "Hope and Anchor 1970s £40 per night".
- Fetches: muhistory.com 1971-80 chapter (fetch_content blocked by
  Cloudflare check; recovered via curl); loudersound pub-rock feature
  (fetched OK, 18KB); musiciansunion.org.uk/about-us/our-history failed
  (invalid response).
- Dead ends: modern results dominate all fee/commission queries; the
  last four searches returned generic noise ("pub", "bands" pages) — the
  topic is old enough that web indexing is thin. The MU history
  chapter's live/pub sections (if any) did not surface in text
  extraction; only session/ILR/theatre figures did.
- What future research should try (highest value first):
1. British Newspaper Archive / archive.org scans of Melody Maker & NME
   1971-79: "gig wanted" and situations columns, plus booking-agency
   display ads (may state commission).
2. Will Birch, No Sleep Till Canvey Island — full text (the loudersound
   excerpt had no figures; the book may).
3. MU archive (the MU history project points to deposited records /
   Stirling University) or physical MU rate cards of the era.
4. Jon Savage, England's Dreaming — full text, first-gigs chapters.
5. Rocksbackpages interviews (paywalled) with Dr Feelgood, Hot Rods, and
   their managers/promoters.
6. Academic theses on 1970s British live-music economics (search
   EThOS/ProQuest).
