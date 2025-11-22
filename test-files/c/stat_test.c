#include <stdio.h>
#include <sys/stat.h>
#include <unistd.h>

int main(void) {
  struct stat st;

  // Stat the current directory
  if (stat(".", &st) != 0) {
    perror("stat");
    return 1;
  }

  printf("%d\n", S_ISDIR(st.st_mode) ? 1 : 0);

  // Create a test file
  FILE *f = fopen("/tmp/stat_test.txt", "w");
  if (!f) {
    perror("fopen");
    return 1;
  }
  fprintf(f, "test\n");
  fclose(f);

  // Stat the file
  if (stat("/tmp/stat_test.txt", &st) != 0) {
    perror("stat file");
    return 1;
  }

  printf("%d\n", S_ISREG(st.st_mode) ? 1 : 0);
  printf("%ld\n", (long)st.st_size);

  // Test fstat
  f = fopen("/tmp/stat_test.txt", "r");
  if (!f) {
    perror("fopen read");
    return 1;
  }

  if (fstat(fileno(f), &st) != 0) {
    perror("fstat");
    fclose(f);
    return 1;
  }

  printf("%d\n", S_ISREG(st.st_mode) ? 1 : 0);
  fclose(f);

  unlink("/tmp/stat_test.txt");

  return 0;
}
