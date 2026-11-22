# NOMS-029: Almost-cookable recipes and pantry substitutions

**Status:** ⚪ Backlog

**Related:** [Kitchen and grocery lists](../implementation-plans/NOMS-018-pantry-meal-planner-grocery-list.md), [optional ingredient handling](NOMS-023-kitchen-ingredient-options.md)

Help cooks find recipes they are close to making and decide whether to shop
for a missing ingredient or use a substitution the recipe author listed.

## Proposed first version

- In Pantry's **What can I cook?** section, offer an **Almost cookable** view
  for owned and collected recipes missing one or two required ingredients.
  Rank recipes with one missing ingredient before those with two. Show which
  ingredients are missing and offer the existing missing-ingredients grocery
  list flow.
- When the selected pantry has an authored substitution, show which missing
  primary ingredient it can replace. Let the cook confirm a swap before
  generating the list. Use the chosen substitution's recipe amount and unit
  wherever the recipe ingredient is displayed in that flow.
- Use the same presence rules in Pantry, recipe detail and grocery creation.
  Check availability by case-insensitive ingredient name and keep optional
  ingredients out of the required shortage count. Continue offering the
  optional-ingredient toggle when generating groceries.

## Acceptance

A recipe missing one required ingredient appears ahead of one missing two.
A recipe missing only an optional garnish stays cookable. Confirming an available
swap removes that requirement from the grocery preview and identifies the
substitution used. Declining the swap keeps the original requirement. Confirmed
choices apply to that list creation and preserve the authored recipe.

Before implementation, settle how repeated ingredients and multiple alternatives
count toward shortages, and how the UI distinguishes a full match from a recipe
that can be made with a substitution.
