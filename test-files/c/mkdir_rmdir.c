#include <stdio.h>
#include <sys/stat.h>
#include <unistd.h>

int main(void) {
  // Create a directory
  if (mkdir("/tmp/test_dir_12345", 0755) != 0) {
    perror("mkdir");
    return 1;
  }

  printf("directory created\n");

  // Check it exists
  struct stat st;
  if (stat("/tmp/test_dir_12345", &st) == 0 && S_ISDIR(st.st_mode)) {
    printf("directory exists\n");
  }

  // Remove the directory
  if (rmdir("/tmp/test_dir_12345") != 0) {
    perror("rmdir");
    return 1;
  }

  printf("directory removed\n");

  // Verify it's gone
  if (stat("/tmp/test_dir_12345", &st) != 0) {
    printf("directory gone\n");
  }

  return 0;
}
