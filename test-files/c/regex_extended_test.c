#include <regex.h>
#include <stdio.h>
#include <string.h>

static void try_match(regex_t *rx, const char *s) {
  regmatch_t matches[1];
  int rc = regexec(rx, s, 1, matches, 0);
  if (rc == 0) {
    printf("match:%s\n", s);
  } else {
    char buf[64];
    regerror(rc, rx, buf, sizeof(buf));
    printf("no_match:%s err=%s\n", s, buf);
  }
}

int main(void) {
  const char *pat = "^abc[0-9][0-9]$";
  regex_t rx;
  int rc = regcomp(&rx, pat, REG_EXTENDED | REG_NOSUB);
  if (rc != 0) {
    char buf[64];
    regerror(rc, &rx, buf, sizeof(buf));
    printf("compile_failed:%s\n", buf);
    return 1;
  }

  try_match(&rx, "abc12");
  try_match(&rx, "abc123");
  try_match(&rx, "nope");
  try_match(&rx, "ABC12");

  regfree(&rx);
  return 0;
}
