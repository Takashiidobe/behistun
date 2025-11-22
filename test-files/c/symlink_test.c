#include <fcntl.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>

int main(void) {
  // Create a target file
  int fd = open("/tmp/symlink_target.txt", O_CREAT | O_WRONLY, 0644);
  if (fd < 0) {
    perror("open");
    return 1;
  }
  write(fd, "target\n", 7);
  close(fd);

  // Create a symbolic link
  if (symlink("/tmp/symlink_target.txt", "/tmp/symlink_link.txt") != 0) {
    perror("symlink");
    unlink("/tmp/symlink_target.txt");
    return 1;
  }

  printf("symlink created\n");

  // Read the link
  char buf[256];
  ssize_t len = readlink("/tmp/symlink_link.txt", buf, sizeof(buf) - 1);
  if (len < 0) {
    perror("readlink");
  } else {
    buf[len] = '\0';
    if (strcmp(buf, "/tmp/symlink_target.txt") == 0) {
      printf("readlink works\n");
    }
  }

  // Clean up
  unlink("/tmp/symlink_link.txt");
  unlink("/tmp/symlink_target.txt");

  return 0;
}
