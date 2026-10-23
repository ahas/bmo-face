<template>
  <div class="flex min-h-dvh items-center justify-center p-4">
    <UCard class="w-full max-w-md">
      <template #header>
        <h1 class="text-xl font-semibold">{{ t("face.edit.login.title") }}</h1>
      </template>

      <form id="editor-sign-in" class="flex flex-col gap-4" @submit.prevent="submit">
        <UFormField :label="t('face.signin.name')" name="name">
          <UInput v-model="name" class="w-full" autocomplete="username" required />
        </UFormField>
        <UFormField :label="t('face.signin.password')" name="password">
          <UInput v-model="password" class="w-full" type="password" autocomplete="current-password" required />
        </UFormField>
        <p v-if="refusal" class="text-sm text-error">{{ refusal }}</p>
      </form>

      <template #footer>
        <div class="flex justify-end">
          <UButton type="submit" form="editor-sign-in" :loading="busy" :label="t('face.signin.submit')" />
        </div>
      </template>
    </UCard>
  </div>
</template>

<script setup lang="ts">
// The editor's login, with a BMO web account (docs/face.md, 「How a screen signs in」).

const props = defineProps<{ login: (name: string, password: string) => Promise<"ok" | "wrong" | "failed"> }>();
const { t } = useI18n();

const name = ref("");
const password = ref("");
const busy = ref(false);
const refusal = ref("");

async function submit(): Promise<void> {
  busy.value = true;
  refusal.value = "";

  const result = await props.login(name.value.trim(), password.value);

  busy.value = false;

  if (result !== "ok") {
    refusal.value = t(`face.edit.login.${result}`);
    password.value = "";
  }
}
</script>
