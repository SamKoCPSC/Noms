# NOMS-031: Community posts, comments and replies

**Status:** ⚪ Backlog

**Related:** [Recipe visibility](NOMS-008-recipe-crud.md), [public Search](NOMS-012-recipe-search.md)

Give cooks a place to share cooking notes, ask questions and discuss recipes
with other members.

## Proposed first version

- Let signed-in members publish text posts on Community Home and comment on
  posts and public recipes. Start with one level of replies to keep
  conversations easy to follow on mobile.
- Show the author, posting time and reply count where useful. Let members edit
  and delete their own contributions, and paginate long discussions.
- Keep recipe discussions within the recipe's visibility rules. Define reporting,
  moderation, posting limits and account-deletion behavior as part of the
  first release.

## Acceptance

A signed-in cook can create a post, another member can comment, and either can
reply. Recipe comments appear on the corresponding public recipe. Only authorized
users can change a contribution. A recipe becoming private also restricts access
to its discussion.

Before implementation, decide whether signed-out visitors can read discussions,
what deletion shows when replies remain, and who handles reports.
