#include <fcntl.h>
#include <stdio.h>
#include <sys/stat.h>
#include <unistd.h>

int main(void) {
  // Create a test file
  int fd = open("/tmp/chmod_test.txt", O_CREAT | O_WRONLY, 0644);
  if (fd < 0) {
    perror("open");
    return 1;
  }
  close(fd);

  // Change permissions
  if (chmod("/tmp/chmod_test.txt", 0755) != 0) {
    perror("chmod");
    unlink("/tmp/chmod_test.txt");
    return 1;
  }

  printf("chmod works\n");

  // Verify with stat
  struct stat st;
  if (stat("/tmp/chmod_test.txt", &st) == 0) {
    if ((st.st_mode & 0777) == 0755) {
      printf("permissions correct\n");
    }
  }

  // Test fchmod
  fd = open("/tmp/chmod_test.txt", O_RDONLY);
  if (fd >= 0) {
    if (fchmod(fd, 0600) == 0) {
      printf("fchmod works\n");
    }
    close(fd);
  }

  unlink("/tmp/chmod_test.txt");

  return 0;
}
