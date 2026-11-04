# Purchased groceries to pantry

Real browser captures using synthetic local accounts. Mark grocery items as
purchased, select **Add purchased to pantry**, choose the destination and exclude
household items before saving. Existing ingredient amounts are kept; new rows
copy grocery quantities and units. The shopping list retains purchase history.

## Mobile

![Select purchased ingredients on mobile](put-away-390.png)

## Desktop

![Select purchased ingredients on desktop](put-away-1440.png)

The [browser regression](../../../../scripts/tests/test-purchased-pantry-browser.py)
verifies this flow, a lost-response retry, preserved quantities, persistence,
selection restrictions, ownership and stale-list rejection with no page errors.
