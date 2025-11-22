#include <spawn.h>
#include <stdio.h>
#include <sys/wait.h>
#include <unistd.h>

extern char **environ;

int main(void) {
  pid_t pid;
  char *argv[] = {"./test-bins/c/true", NULL};
  int rc = posix_spawn(&pid, argv[0], NULL, NULL, argv, environ);
  if (rc != 0) {
    printf("spawn rc=%d\n", rc);
    return 1;
  }
  int status = 0;
  if (waitpid(pid, &status, 0) < 0) {
    perror("waitpid");
    return 1;
  }
  printf("exit=%d\n", WEXITSTATUS(status));
  return WEXITSTATUS(status);
}
