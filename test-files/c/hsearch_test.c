#include <search.h>
#include <stdio.h>
#include <string.h>

static const char *words[] = {"alpha", "bravo", "charlie", "delta", NULL};

int main(void) {
  if (hcreate(8) == 0) {
    perror("hcreate");
    return 1;
  }

  for (int i = 0; words[i]; i++) {
    ENTRY e = {.key = (char *)words[i], .data = (void *)(long)i};
    if (hsearch(e, ENTER) == NULL) {
      perror("hsearch enter");
      return 1;
    }
  }

  for (int i = 0; i < 5; i++) {
    ENTRY query = {.key = (i < 4) ? (char *)words[i] : "missing"};
    ENTRY *res = hsearch(query, FIND);
    printf("%s -> %s\n", query.key, res ? (char *)res->key : "(not found)");
  }
  hdestroy();
  return 0;
}
