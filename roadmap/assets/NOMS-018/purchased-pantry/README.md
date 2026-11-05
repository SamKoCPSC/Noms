# Purchased groceries to pantry

Real browser captures using synthetic local accounts. Mark grocery items as
purchased, select **Add purchased to pantry**, choose the destination and exclude
household items before saving. Existing ingredient amounts are kept; new rows
copy grocery quantities and units. Choose to keep selected rows on the grocery list (default) or remove them after the pantry save.

## Mobile

![Select purchased ingredients on mobile](put-away-390.png)

## Desktop

![Select purchased ingredients on desktop](put-away-1440.png)

The [browser regression](../../../../scripts/tests/test-purchased-pantry-browser.py)
verifies this flow, a lost-response retry, preserved quantities, persistence,
selection restrictions, ownership and stale-list rejection with no page errors.

## Remove after adding

Only selected purchased rows are removed. A lost-response retry confirms the
original save without adding another pantry entry or removing extra rows.

![Remove selected ingredients after adding](put-away-remove-390.png)
