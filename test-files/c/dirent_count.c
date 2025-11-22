#include <dirent.h>
#include <stdio.h>

int main(void) {
  DIR *d = opendir(".");
  if (!d) {
    perror("opendir");
    return 1;
  }
  int count = 0;
  struct dirent *ent;
  while ((ent = readdir(d)) != NULL) {
    if (ent->d_name[0] == '.')
      continue;
    ++count;
  }
  closedir(d);
  printf("%d\n", count);
  return count & 0xff;
}
