# Exposure record template

Use one record per person or agent and per task. File the completed record **privately**, outside
the repository. The public repository records only the resulting eligibility statement.
Categories E0 to EU are defined in the [admission policy](admission-policy.md#4-exposure-records).

```text
Exposure record (version 0.1.0)

Subject:            <person or agent name / model identifier>
Role requested:     <analyst | reviewer | test author | implementer | integrator>
Task and area(s):   <task ID; kernel areas, e.g. "plane cuts", "triangulation">
Date and base SHA:  <date; repository commit the task starts from>
Fresh context:      <yes/no; how the context was started; inherited history, if any>

Known prior exposure, per area:
  Area                | Category (E0-E4, EU) | What was seen (kind, not content) | When / where
  --------------------|----------------------|-----------------------------------|-------------
  <area>              | <code>               | <e.g. "source of a permissively licensed kernel" | <chat/date>

Sources consulted for this task so far (kinds and identifiers only):
  <list, or "none beyond the task's allowlist">

Unknowns that cannot be established (for example unrecorded training content): <statement>

Declaration: the facts above are complete to the best of the subject's knowledge.
Eligibility decision (reviewer): <eligible | not eligible | needs owner decision> for <areas>
Reason:                          <one or two sentences, no restricted content>
Owner-accepted risk (if any):    <scope, date, reference to the owner's recorded decision>
```

Rules:

- Record kinds of material, never their content. Do not paste restricted text into the record.
- A record is valid for the listed areas and task only.
- Any exposure after the record is filed is added at once; a blocking category ends the task.
- An empty record is not evidence of independence. It is a statement of what is known.
