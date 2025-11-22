#include <fcntl.h>
#include <stdio.h>
#include <sys/stat.h>
#include <unistd.h>

int main(void) {
  // Save original umask
  mode_t old_umask = umask(0);
  umask(old_umask);

  printf("umask works\n");

  // Set umask to 022
  umask(0022);

  // Create file with 0666 permissions
  int fd = open("/tmp/umask_test.txt", O_CREAT | O_WRONLY, 0666);
  if (fd < 0) {
    perror("open");
    return 1;
  }
  close(fd);

  // Check actual permissions (should be 0644 due to umask)
  struct stat st;
  if (stat("/tmp/umask_test.txt", &st) == 0) {
    mode_t mode = st.st_mode & 0777;
    if (mode == 0644) {
      printf("umask applied correctly\n");
    } else {
      printf("mode: %o\n", mode);
    }
  }

  unlink("/tmp/umask_test.txt");

  // Restore original umask
  umask(old_umask);

  return 0;
}
