#include <stdio.h>
#include <string.h>

int main(void) {
  char buf[] = "one,two,three";
  int count = 0;
  for (char *tok = strtok(buf, ","); tok; tok = strtok(NULL, ",")) {
    printf("%s ", tok);
    ++count;
  }
  printf("\n");
  const char *hay = "abcdefthreegh";
  const char *needle = "three";
  const char *p = strstr(hay, needle);
  int pos = p ? (int)(p - hay) : -1;
  printf("%d\n", pos);
  return count + pos;
}
