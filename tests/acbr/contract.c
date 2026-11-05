/* Test double for the documented MT/cdecl ABI. NOT ACBr; never packaged. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
typedef struct { char path[4096]; int loaded; int validated; int reject; } Instance;
int NFE_Inicializar(void **handle,const char *config,const char *key) {
    if (!config || !key) return -1;
    *handle=calloc(1,sizeof(Instance)); return *handle ? 0 : -1;
}
int NFE_Finalizar(void *handle) { free(handle); return 0; }
int NFE_LimparLista(void *handle) { ((Instance*)handle)->loaded=0; return 0; }
int NFE_ConfigGravarValor(void *handle,const char *section,const char *key,const char *value) {
    if (!strcmp(section,"DANFE") && !strcmp(key,"PathPDF")) snprintf(((Instance*)handle)->path,4096,"%s",value);
    return 0;
}
int NFE_CarregarXML(void *handle,const char *xml) {
    Instance *i=handle; i->loaded=1; i->reject=strstr(xml,"REJECT")!=NULL; return 0;
}
int NFE_Validar(void *handle) {
    Instance *i=handle; if (!i->loaded || i->reject) return -11;
    i->validated=1;return 0;
}
int NFE_ImprimirPDF(void *handle) {
    Instance *i=handle;if (!i->validated) return -10;
    char path[4200];snprintf(path,sizeof(path),"%s/contract.pdf",i->path);
    FILE *f=fopen(path,"wb");if(!f)return -10;
    fputs("%PDF-1.4\ncontract-test-only\n%%EOF",f);fclose(f);return 0;
}
static int output(const char *text,char *buffer,int *length) {
    int needed=(int)strlen(text)+1;
    if (*length>=needed) memcpy(buffer,text,needed);
    *length=needed;return 0;
}
int NFE_UltimoRetorno(void *handle,char *buffer,int *length) {
    (void)handle;return output("fixture validation error",buffer,length);
}
int NFE_Versao(void *handle,char *buffer,int *length) {
    (void)handle;return output("contract-test-only",buffer,length);
}
