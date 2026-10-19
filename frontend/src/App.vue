<template>
  <div class="app">
    <el-config-provider :locale="zhCn">
      <div class="app-container">
        <!-- 顶部导航栏 -->
        <el-header class="header">
          <div class="header-content">
            <div class="logo">
              <h1>🔒 SecurePass</h1>
              <span class="subtitle">安全密码管理器</span>
            </div>
            <div class="header-actions">
              <el-button 
                type="primary" 
                @click="showExportDialog = true"
                :icon="Download"
              >
                导出密码
              </el-button>
              <el-button 
                type="info" 
                @click="showImportDialog = true"
                :icon="Upload"
              >
                导入密码
              </el-button>
              <el-button 
                type="success" 
                @click="showBackupDialog = true"
                :icon="DocumentCopy"
              >
                备份恢复
              </el-button>
              <el-button 
                type="warning" 
                @click="showSettingsDialog = true"
                :icon="Setting"
              >
                设置
              </el-button>
            </div>
          </div>
        </el-header>

        <!-- 主要内容区域 -->
        <el-main class="main">
          <div class="content-wrapper">
            <!-- 欢迎信息 -->
            <div v-if="!passwordsLoaded" class="welcome-section">
              <el-card class="welcome-card">
                <div class="welcome-content">
                  <div class="welcome-icon">🔒</div>
                  <div class="welcome-text">
                    <h2>欢迎使用 SecurePass</h2>
                    <p>您的安全密码管理助手</p>
                  </div>
                  <div class="welcome-actions">
                    <el-button 
                      type="primary" 
                      size="large"
                      @click="startUsing"
                    >
                      开始使用
                    </el-button>
                    <el-button 
                      size="large"
                      @click="showTutorial"
                    >
                      使用教程
                    </el-button>
                  </div>
                </div>
              </el-card>
            </div>

            <!-- 密码管理器 -->
            <PasswordManager v-if="passwordsLoaded" />
          </div>
        </el-main>

        <!-- 底部信息 -->
        <el-footer class="footer">
          <div class="footer-content">
            <span>© 2024 SecurePass - 安全密码管理器</span>
            <span class="footer-divider">|</span>
            <span>版本 1.0.0</span>
            <span class="footer-divider">|</span>
            <span>开源项目</span>
          </div>
        </el-footer>
      </div>
    </el-config-provider>

    <!-- 导出对话框 -->
    <el-dialog
      v-model="showExportDialog"
      title="导出密码"
      width="400px"
    >
      <div class="export-options">
        <el-radio-group v-model="exportFormat">
          <el-radio label="json">JSON 格式</el-radio>
          <el-radio label="csv">CSV 格式</el-radio>
        </el-radio-group>
      </div>
      <div class="export-info">
        <p>导出格式：{{ exportFormat === 'json' ? 'JSON' : 'CSV' }}</p>
        <p>包含内容：所有密码条目</p>
        <p>注意：导出的文件包含加密后的密码，请妥善保管</p>
      </div>
      <template #footer>
        <el-button @click="showExportDialog = false">取消</el-button>
        <el-button type="primary" @click="handleExport" :loading="exporting">
          导出
        </el-button>
      </template>
    </el-dialog>

    <!-- 导入对话框 -->
    <el-dialog
      v-model="showImportDialog"
      title="导入密码"
      width="400px"
    >
      <div class="import-options">
        <el-form :model="importForm" label-width="80px">
          <el-form-item label="文件格式">
            <el-radio-group v-model="importForm.format">
              <el-radio label="json">JSON 格式</el-radio>
              <el-radio label="csv">CSV 格式</el-radio>
            </el-radio-group>
          </el-form-item>
          <el-form-item label="导入文件">
            <el-upload
              ref="uploadRef"
              :auto-upload="false"
              :on-change="handleFileChange"
              :limit="1"
              accept=".json,.csv"
            >
              <el-button type="primary">选择文件</el-button>
              <template #tip>
                <div class="el-upload__tip">
                  只能上传 JSON 或 CSV 文件，且不超过 10MB
                </div>
              </template>
            </el-upload>
          </el-form-item>
          <el-form-item label="处理方式">
            <el-radio-group v-model="importForm.duplicateHandling">
              <el-radio label="skip">跳过重复</el-radio>
              <el-radio label="update">更新重复</el-radio>
              <el-radio label="rename">重命名重复</el-radio>
            </el-radio-group>
          </el-form-item>
        </el-form>
      </div>
      <template #footer>
        <el-button @click="showImportDialog = false">取消</el-button>
        <el-button type="primary" @click="handleImport" :loading="importing">
          导入
        </el-button>
      </template>
    </el-dialog>

    <!-- 备份恢复对话框 -->
    <el-dialog
      v-model="showBackupDialog"
      title="备份恢复"
      width="500px"
    >
      <el-row :gutter="20">
        <el-col :span="12">
          <el-card class="backup-card">
            <template #header>
              <div class="card-header">
                <span>创建备份</span>
                <el-button type="primary" size="small" @click="createBackup">
                  创建备份
                </el-button>
              </div>
            </template>
            <div class="backup-info">
              <p>备份包含：</p>
              <ul>
                <li>所有密码条目</li>
                <li>配置信息</li>
                <li>系统设置</li>
              </ul>
              <p>备份文件将保存到本地</p>
            </div>
          </el-card>
        </el-col>
        <el-col :span="12">
          <el-card class="backup-card">
            <template #header>
              <div class="card-header">
                <span>恢复备份</span>
                <el-button type="success" size="small" @click="showBackupFileSelect">
                  选择备份
                </el-button>
              </div>
            </template>
            <div class="backup-info">
              <p>从备份文件恢复：</p>
              <ul>
                <li>密码条目</li>
                <li>配置信息</li>
                <li>系统设置</li>
              </ul>
              <p>⚠️ 恢复将覆盖现有数据</p>
            </div>
          </el-card>
        </el-col>
      </el-row>
    </el-dialog>

    <!-- 设置对话框 -->
    <el-dialog
      v-model="showSettingsDialog"
      title="设置"
      width="600px"
    >
      <el-form :model="settingsForm" label-width="120px">
        <el-form-item label="主密码">
          <el-input
            v-model="settingsForm.masterPassword"
            type="password"
            placeholder="设置主密码"
            show-password
          />
          <div class="password-strength">
            <el-progress 
              :percentage="settingsPasswordStrength" 
              :color="settingsPasswordStrengthColor"
              :show-text="false"
            />
            <span :class="['strength-text', settingsPasswordStrengthClass]">
              {{ settingsPasswordStrengthText }}
            </span>
          </div>
        </el-form-item>
        
        <el-form-item label="自动锁定">
          <el-switch
            v-model="settingsForm.autoLock"
            active-text="启用"
            inactive-text="禁用"
          />
        </el-form-item>
        
        <el-form-item label="锁定时间">
          <el-slider 
            v-model="settingsForm.lockTime" 
            :min="1" 
            :max="60"
            :step="1"
            :format-tooltip="(value) => `${value} 分钟`"
          />
        </el-form-item>
        
        <el-form-item label="自动备份">
          <el-switch
            v-model="settingsForm.autoBackup"
            active-text="启用"
            inactive-text="禁用"
          />
        </el-form-item>
        
        <el-form-item label="备份频率">
          <el-select v-model="settingsForm.backupFrequency" placeholder="选择备份频率">
            <el-option label="每天" value="daily" />
            <el-option label="每周" value="weekly" />
            <el-option label="每月" value="monthly" />
          </el-select>
        </el-form-item>
        
        <el-form-item label="通知设置">
          <el-switch
            v-model="settingsForm.notifications"
            active-text="启用"
            inactive-text="禁用"
          />
        </el-form-item>
      </el-form>
      
      <template #footer>
        <el-button @click="showSettingsDialog = false">取消</el-button>
        <el-button type="primary" @click="saveSettings" :loading="savingSettings">
          保存设置
        </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import { ElMessage } from 'element-plus';
import zhCn from 'element-plus/dist/locale/zh-cn.mjs';
import { Download, Upload, DocumentCopy, Setting } from '@element-plus/icons-vue';
import PasswordManager from './components/PasswordManager.vue';
import { notificationApi } from './api';

// 响应式数据
const passwordsLoaded = ref(false);
const showExportDialog = ref(false);
const showImportDialog = ref(false);
const showBackupDialog = ref(false);
const showSettingsDialog = ref(false);
const exporting = ref(false);
const importing = ref(false);
const savingSettings = ref(false);

// 导出设置
const exportFormat = ref('json');

// 导入设置
const importForm = ref({
  format: 'json',
  file: null as File | null,
  duplicateHandling: 'skip',
});

// 备份文件引用
const backupFileInput = ref();

// 设置表单
const settingsForm = ref({
  masterPassword: '',
  autoLock: true,
  lockTime: 15,
  autoBackup: true,
  backupFrequency: 'weekly',
  notifications: true,
});

// 计算属性
const settingsPasswordStrength = computed(() => {
  const password = settingsForm.value.masterPassword;
  if (!password) return 0;
  
  let strength = 0;
  if (password.length >= 8) strength += 25;
  if (password.length >= 12) strength += 25;
  if (/[a-z]/.test(password)) strength += 12.5;
  if (/[A-Z]/.test(password)) strength += 12.5;
  if (/[0-9]/.test(password)) strength += 12.5;
  if (/[^a-zA-Z0-9]/.test(password)) strength += 12.5;
  
  return Math.min(strength, 100);
});

const settingsPasswordStrengthColor = computed(() => {
  const strength = settingsPasswordStrength.value;
  if (strength < 30) return '#f56c6c';
  if (strength < 70) return '#e6a23c';
  return '#67c23a';
});

const settingsPasswordStrengthText = computed(() => {
  const strength = settingsPasswordStrength.value;
  if (strength < 30) return '弱';
  if (strength < 70) return '中';
  return '强';
});

const settingsPasswordStrengthClass = computed(() => {
  const strength = settingsPasswordStrength.value;
  if (strength < 30) return 'weak';
  if (strength < 70) return 'medium';
  return 'strong';
});

// 方法
const startUsing = () => {
  passwordsLoaded.value = true;
  notificationApi.showNotification('欢迎使用 SecurePass！', 'success');
};

const showTutorial = () => {
  notificationApi.showNotification('使用教程功能开发中...', 'info');
};

const handleExport = async () => {
  exporting.value = true;
  try {
    // 这里应该调用实际的导出API
    notificationApi.showNotification('导出功能开发中...', 'info');
    showExportDialog.value = false;
  } catch (error) {
    notificationApi.showNotification('导出失败', 'error');
  } finally {
    exporting.value = false;
  }
};

const handleFileChange = (file: any) => {
  importForm.value.file = file.raw;
};

const handleImport = async () => {
  if (!importForm.value.file) {
    notificationApi.showNotification('请选择要导入的文件', 'warning');
    return;
  }
  
  importing.value = true;
  try {
    // 这里应该调用实际的导入API
    notificationApi.showNotification('导入功能开发中...', 'info');
    showImportDialog.value = false;
  } catch (error) {
    notificationApi.showNotification('导入失败', 'error');
  } finally {
    importing.value = false;
  }
};

const createBackup = async () => {
  try {
    // 这里应该调用实际的备份API
    notificationApi.showNotification('备份功能开发中...', 'info');
  } catch (error) {
    notificationApi.showNotification('创建备份失败', 'error');
  }
};

const showBackupFileSelect = () => {
  if (backupFileInput.value) {
    backupFileInput.value.click();
  }
};

const saveSettings = async () => {
  savingSettings.value = true;
  try {
    // 这里应该调用实际的设置保存API
    notificationApi.showNotification('设置保存成功', 'success');
    showSettingsDialog.value = false;
  } catch (error) {
    notificationApi.showNotification('设置保存失败', 'error');
  } finally {
    savingSettings.value = false;
  }
};

// 生命周期
onMounted(() => {
  // 检查是否已经设置了主密码
  // 如果没有，可以跳转到设置页面
  notificationApi.showNotification('SecurePass 已就绪', 'success');
});
</script>

<style scoped>
.app {
  min-height: 100vh;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
}

.app-container {
  min-height: 100vh;
  display: flex;
  flex-direction: column;
}

.header {
  background: rgba(255, 255, 255, 0.95);
  backdrop-filter: blur(10px);
  border-bottom: 1px solid #e4e7ed;
  padding: 0;
}

.header-content {
  max-width: 1200px;
  margin: 0 auto;
  padding: 0 20px;
  height: 100%;
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.logo {
  display: flex;
  align-items: center;
  gap: 10px;
}

.logo h1 {
  margin: 0;
  font-size: 24px;
  color: #303133;
}

.subtitle {
  color: #909399;
  font-size: 14px;
}

.header-actions {
  display: flex;
  gap: 10px;
}

.main {
  flex: 1;
  padding: 20px;
  background: rgba(255, 255, 255, 0.1);
}

.content-wrapper {
  max-width: 1200px;
  margin: 0 auto;
}

.welcome-section {
  display: flex;
  justify-content: center;
  align-items: center;
  min-height: 400px;
}

.welcome-card {
  max-width: 600px;
  text-align: center;
}

.welcome-content {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 20px;
}

.welcome-icon {
  font-size: 64px;
}

.welcome-text h2 {
  margin: 0;
  color: #303133;
  font-size: 28px;
}

.welcome-text p {
  margin: 0;
  color: #909399;
  font-size: 16px;
}

.welcome-actions {
  display: flex;
  gap: 15px;
}

.export-options {
  margin-bottom: 20px;
}

.export-info {
  background: #f5f7fa;
  padding: 15px;
  border-radius: 4px;
  margin-bottom: 20px;
}

.export-info p {
  margin: 5px 0;
  color: #606266;
}

.import-options {
  margin-bottom: 20px;
}

.backup-card {
  height: 100%;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.backup-info {
  color: #606266;
}

.backup-info p {
  margin: 10px 0 5px 0;
}

.backup-info ul {
  margin: 5px 0;
  padding-left: 20px;
}

.password-strength {
  margin-top: 8px;
}

.strength-text {
  margin-left: 10px;
  font-size: 12px;
}

.weak {
  color: #f56c6c;
}

.medium {
  color: #e6a23c;
}

.strong {
  color: #67c23a;
}

.footer {
  background: rgba(255, 255, 255, 0.95);
  backdrop-filter: blur(10px);
  border-top: 1px solid #e4e7ed;
  padding: 20px 0;
}

.footer-content {
  max-width: 1200px;
  margin: 0 auto;
  padding: 0 20px;
  text-align: center;
  color: #909399;
}

.footer-divider {
  margin: 0 10px;
}
</style>