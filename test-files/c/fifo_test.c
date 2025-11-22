#include <assert.h>
#include <fcntl.h>
#include <stdio.h>
#include <sys/stat.h>
#include <unistd.h>

int main(void) {
  const char *path = "/tmp/tmp_fifo";
  unlink(path);
  assert(mkfifo(path, 0600) == 0);
  pid_t pid = fork();
  assert(pid >= 0);
  if (pid == 0) {
    int fd = open(path, O_WRONLY);
    assert(fd >= 0);
    const char *msg = "fifo";
    write(fd, msg, 4);
    close(fd);
    _exit(0);
  } else {
    int fd = open(path, O_RDONLY);
    assert(fd >= 0);
    char buf[8] = {0};
    assert(read(fd, buf, sizeof(buf)) == 4);
    printf("%s\n", buf);
    close(fd);
    unlink(path);
    return 0;
  }
}
