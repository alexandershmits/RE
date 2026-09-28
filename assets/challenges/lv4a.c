#include <stdio.h>
#include <string.h>
int main(void){ char buf[32];
  printf("password: "); scanf("%31s", buf);
  char real[32]; int n = strlen(buf);
  for (int i = 0; i < n; i++) real[i] = buf[i] + 1;  /* each char shifted by +1 */
  real[n] = 0;
  if (!strcmp(real, "sfdmfu")) { puts("Correct! FLAG{lv4a_shift}"); return 0; }
  puts("Wrong"); return 1; }