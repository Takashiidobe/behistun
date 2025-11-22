#include <fcntl.h>
#include <stdio.h>
#include <sys/stat.h>
#include <unistd.h>

int main(void) {
  // Create a test file
  int fd = open("/tmp/link_orig.txt", O_CREAT | O_WRONLY, 0644);
  if (fd < 0) {
    perror("open");
    return 1;
  }
  write(fd, "original\n", 9);
  close(fd);

  // Create a hard link
  if (link("/tmp/link_orig.txt", "/tmp/link_hard.txt") != 0) {
    perror("link");
    unlink("/tmp/link_orig.txt");
    return 1;
  }

  printf("hard link created\n");

  // Verify both files have same inode
  struct stat st1, st2;
  stat("/tmp/link_orig.txt", &st1);
  stat("/tmp/link_hard.txt", &st2);

  if (st1.st_ino == st2.st_ino) {
    printf("same inode\n");
  }

  // Check link count
  if (st1.st_nlink == 2) {
    printf("link count is 2\n");
  }

  // Clean up
  unlink("/tmp/link_orig.txt");
  unlink("/tmp/link_hard.txt");

  return 0;
}
