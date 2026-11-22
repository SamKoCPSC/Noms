# NOMS-030: Weekly meal planning

**Status:** ⚪ Backlog

**Related:** [Kitchen and grocery lists](../implementation-plans/NOMS-018-pantry-meal-planner-grocery-list.md)

Plan what to cook for the week and make one shopping list from those meals.

## Proposed first version

- Save a private weekly plan with recipe entries assigned to days. Let users
  add, move and remove entries and set servings for each meal.
- Generate one editable grocery list from the planned meals and serving counts.
  Account for each scheduled meal, including when the same recipe is planned
  more than once. Reuse the existing name and unit matching rules.
- Offer the existing pantry selection and missing-only mode, along with the
  option to exclude optional ingredients. Save the list as an independent copy
  so it can be edited and checked off during shopping.

## Acceptance

A user can reload a saved week, change a meal's servings and generate a list
that reflects the new counts. When a recipe appears twice, both meals contribute
to the total. Selecting a pantry can omit ingredients marked available there.
Later edits to the plan do not change a saved grocery list.

Define the week picker and how a saved plan displays recipes that later become
unavailable before implementation.
