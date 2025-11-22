#include <stdio.h>
#include <stdlib.h>

struct node {
  int data;
  struct node *next;
};

struct node *create_node(int data) {
  struct node *n = malloc(sizeof(struct node));
  n->data = data;
  n->next = NULL;
  return n;
}

int main(void) {
  struct node *head = create_node(10);
  head->next = create_node(20);
  head->next->next = create_node(30);
  head->next->next->next = create_node(40);

  // Traverse and print
  struct node *curr = head;
  while (curr != NULL) {
    printf("%d\n", curr->data);
    curr = curr->next;
  }

  // Free memory
  curr = head;
  while (curr != NULL) {
    struct node *tmp = curr;
    curr = curr->next;
    free(tmp);
  }

  return 0;
}
