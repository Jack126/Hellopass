<template>
  <div class="password-manager">
    <el-card class="main-card">
      <template #header>
        <div class="card-header">
          <h2>🔒 SecurePass 密码管理器</h2>
          <el-button 
            type="primary" 
            @click="showAddDialog = true"
            :icon="Plus"
          >
            添加密码
          </el-button>
        </div>
      </template>

      <!-- 搜索栏 -->
      <div class="search-bar">
        <el-input
          v-model="searchQuery"
          placeholder="搜索密码..."
          class="search-input"
          :prefix-icon="Search"
          clearable
          @input="handleSearch"
        />
        <el-select v-model="selectedCategory" placeholder="分类" clearable @change="handleSearch">
          <el-option label="全部" value="" />
          <el-option label="社交" value="social" />
          <el-option label="工作" value="work" />
          <el-option label="购物" value="shopping" />
          <el-option label="娱乐" value="entertainment" />
          <el-option label="其他" value="other" />
        </el-select>
      </div>

      <!-- 密码列表 -->
      <div v-if="loading" class="loading">
        <el-skeleton :rows="5" animated />
      </div>
      
      <div v-else-if="passwords.length === 0" class="empty-state">
        <el-empty description="暂无密码">
          <el-button type="primary" @click="showAddDialog = true">
            添加第一个密码
          </el-button>
        </el-empty>
      </div>
      
      <div v-else class="password-list">
        <div 
          v-for="password in filteredPasswords" 
          :key="password.id"
          class="password-item"
          @click="selectPassword(password)"
        >
          <div class="password-info">
            <div class="site-info">
              <div class="site-name">{{ password.site_name }}</div>
              <div class="site-url">{{ password.site_url }}</div>
            </div>
            <div class="password-meta">
              <div class="username">{{ password.username }}</div>
              <div class="tags">
                <el-tag 
                  v-for="tag in password.tags" 
                  :key="tag" 
                  size="small"
                  class="tag"
                >
                  {{ tag }}
                </el-tag>
              </div>
            </div>
          </div>
          <div class="password-actions">
            <el-button 
              type="primary" 
              size="small" 
              @click.stop="copyPassword(password)"
            >
              复制密码
            </el-button>
            <el-button 
              type="info" 
              size="small" 
              @click.stop="editPassword(password)"
            >
              编辑
            </el-button>
            <el-button 
              type="danger" 
              size="small" 
              @click.stop="deletePassword(password)"
            >
              删除
            </el-button>
          </div>
        </div>
      </div>

      <!-- 统计信息 -->
      <div class="stats">
        <el-row :gutter="20">
          <el-col :span="6">
            <el-card class="stat-card">
              <div class="stat-content">
                <div class="stat-number">{{ passwords.length }}</div>
                <div class="stat-label">总密码数</div>
              </div>
            </el-card>
          </el-col>
          <el-col :span="6">
            <el-card class="stat-card">
              <div class="stat-content">
                <div class="stat-number">{{ categoryStats.social }}</div>
                <div class="stat-label">社交</div>
              </div>
            </el-card>
          </el-col>
          <el-col :span="6">
            <el-card class="stat-card">
              <div class="stat-content">
                <div class="stat-number">{{ categoryStats.work }}</div>
                <div class="stat-label">工作</div>
              </div>
            </el-card>
          </el-col>
          <el-col :span="6">
            <el-card class="stat-card">
              <div class="stat-content">
                <div class="stat-number">{{ categoryStats.shopping }}</div>
                <div class="stat-label">购物</div>
              </div>
            </el-card>
          </el-col>
        </el-row>
      </div>
    </el-card>

    <!-- 添加/编辑密码对话框 -->
    <el-dialog
      v-model="showAddDialog"
      :title="editingPassword ? '编辑密码' : '添加密码'"
      width="500px"
    >
      <el-form 
        ref="passwordFormRef"
        :model="passwordForm"
        :rules="passwordRules"
        label-width="100px"
      >
        <el-form-item label="网站名称" prop="site_name">
          <el-input v-model="passwordForm.site_name" placeholder="请输入网站名称" />
        </el-form-item>
        
        <el-form-item label="网站地址" prop="site_url">
          <el-input v-model="passwordForm.site_url" placeholder="请输入网站地址" />
        </el-form-item>
        
        <el-form-item label="用户名" prop="username">
          <el-input v-model="passwordForm.username" placeholder="请输入用户名" />
        </el-form-item>
        
        <el-form-item label="密码" prop="password">
          <div style="display: flex; gap: 8px; width: 100%;">
            <el-input
              v-model="passwordForm.password"
              type="password"
              placeholder="请输入密码"
              show-password
              style="flex: 1;"
            />
            <el-button @click="generatePassword">生成</el-button>
          </div>
          <div class="password-strength">
            <el-progress 
              :percentage="passwordStrength" 
              :color="passwordStrengthColor"
              :show-text="false"
            />
            <span :class="['strength-text', passwordStrengthClass]">
              {{ passwordStrengthText }}
            </span>
          </div>
        </el-form-item>
        
        <el-form-item label="备注">
          <el-input 
            v-model="passwordForm.notes" 
            type="textarea" 
            placeholder="请输入备注信息"
          />
        </el-form-item>
        
        <el-form-item label="标签">
          <el-select
            v-model="passwordForm.tags"
            multiple
            filterable
            allow-create
            default-first-option
            placeholder="请选择或输入标签"
          >
            <el-option
              v-for="tag in availableTags"
              :key="tag"
              :label="tag"
              :value="tag"
            />
          </el-select>
        </el-form-item>
        
        <el-form-item label="分类">
          <el-select v-model="passwordForm.category" placeholder="请选择分类">
            <el-option label="社交" value="social" />
            <el-option label="工作" value="work" />
            <el-option label="购物" value="shopping" />
            <el-option label="娱乐" value="entertainment" />
            <el-option label="其他" value="other" />
          </el-select>
        </el-form-item>
        
        <el-form-item label="优先级">
          <el-slider v-model="passwordForm.priority" :min="1" :max="5" />
        </el-form-item>
      </el-form>
      
      <template #footer>
        <el-button @click="showAddDialog = false">取消</el-button>
        <el-button type="primary" @click="savePassword" :loading="saving">
          {{ editingPassword ? '保存' : '添加' }}
        </el-button>
      </template>
    </el-dialog>

    <!-- 密码详情对话框 -->
    <el-dialog
      v-model="showDetailDialog"
      title="密码详情"
      width="600px"
    >
      <div v-if="selectedPassword">
        <el-descriptions :column="1" border>
          <el-descriptions-item label="网站名称">
            {{ selectedPassword.site_name }}
          </el-descriptions-item>
          <el-descriptions-item label="网站地址">
            <a :href="selectedPassword.site_url" target="_blank" class="link">
              {{ selectedPassword.site_url }}
            </a>
          </el-descriptions-item>
          <el-descriptions-item label="用户名">
            {{ selectedPassword.username }}
          </el-descriptions-item>
          <el-descriptions-item label="密码">
            <el-input
              v-model="decryptedPassword"
              type="password"
              readonly
              show-password
              class="password-input"
            >
              <template #append>
                <el-button @click="copyToClipboard(decryptedPassword)">
                  复制
                </el-button>
              </template>
            </el-input>
          </el-descriptions-item>
          <el-descriptions-item label="备注">
            {{ selectedPassword.notes || '无' }}
          </el-descriptions-item>
          <el-descriptions-item label="标签">
            <el-tag
              v-for="tag in selectedPassword.tags"
              :key="tag"
              size="small"
              class="tag"
            >
              {{ tag }}
            </el-tag>
          </el-descriptions-item>
          <el-descriptions-item label="分类">
            {{ getCategoryName(selectedPassword.category) }}
          </el-descriptions-item>
          <el-descriptions-item label="优先级">
            {{ selectedPassword.priority }}
          </el-descriptions-item>
          <el-descriptions-item label="创建时间">
            {{ formatDate(selectedPassword.created_at) }}
          </el-descriptions-item>
          <el-descriptions-item label="更新时间">
            {{ formatDate(selectedPassword.updated_at) }}
          </el-descriptions-item>
        </el-descriptions>
      </div>
      
      <template #footer>
        <el-button @click="showDetailDialog = false">关闭</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, nextTick } from 'vue';
import { ElMessage, ElMessageBox } from 'element-plus';
import { Plus, Search } from '@element-plus/icons-vue';
import { 
  passwordApi, 
  notificationApi,
  type PasswordEntry, 
  type NewPassword,
  type UpdatePassword 
} from '@/api';

// 响应式数据
const loading = ref(false);
const saving = ref(false);
const showAddDialog = ref(false);
const showDetailDialog = ref(false);
const searchQuery = ref('');
const selectedCategory = ref('');
const passwords = ref<PasswordEntry[]>([]);
const selectedPassword = ref<PasswordEntry | null>(null);
const decryptedPassword = ref('');
const editingPassword = ref<PasswordEntry | null>(null);

// 表单数据
const passwordForm = ref({
  site_name: '',
  site_url: '',
  username: '',
  password: '',
  notes: '',
  tags: [] as string[],
  category: 'other',
  priority: 3,
});

// 表单验证规则
const passwordFormRef = ref();
const passwordRules = {
  site_name: [
    { required: true, message: '请输入网站名称', trigger: 'blur' },
    { min: 2, max: 50, message: '长度在 2 到 50 个字符', trigger: 'blur' }
  ],
  site_url: [
    { required: true, message: '请输入网站地址', trigger: 'blur' }
  ],
  username: [
    { required: true, message: '请输入用户名', trigger: 'blur' }
  ],
  password: [
    { required: true, message: '请输入密码', trigger: 'blur' },
    { min: 6, message: '密码长度至少6位', trigger: 'blur' }
  ],
};

// 可用标签
const availableTags = ref(['重要', '个人', '工作', '常用', '临时']);

// 计算属性
const filteredPasswords = computed(() => {
  let filtered = passwords.value;
  
  if (searchQuery.value) {
    filtered = filtered.filter(password => 
      password.site_name.toLowerCase().includes(searchQuery.value.toLowerCase()) ||
      password.username.toLowerCase().includes(searchQuery.value.toLowerCase()) ||
      password.tags.some(tag => tag.toLowerCase().includes(searchQuery.value.toLowerCase()))
    );
  }
  
  if (selectedCategory.value) {
    filtered = filtered.filter(password => password.category === selectedCategory.value);
  }
  
  return filtered;
});

const categoryStats = computed(() => {
  return {
    social: passwords.value.filter(p => p.category === 'social').length,
    work: passwords.value.filter(p => p.category === 'work').length,
    shopping: passwords.value.filter(p => p.category === 'shopping').length,
    other: passwords.value.filter(p => p.category === 'other').length,
  };
});

const passwordStrength = computed(() => {
  const password = passwordForm.value.password;
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

const passwordStrengthColor = computed(() => {
  const strength = passwordStrength.value;
  if (strength < 30) return '#f56c6c';
  if (strength < 70) return '#e6a23c';
  return '#67c23a';
});

const passwordStrengthText = computed(() => {
  const strength = passwordStrength.value;
  if (strength < 30) return '弱';
  if (strength < 70) return '中';
  return '强';
});

const passwordStrengthClass = computed(() => {
  const strength = passwordStrength.value;
  if (strength < 30) return 'weak';
  if (strength < 70) return 'medium';
  return 'strong';
});

const randInt = (max: number) => {
  const limit = Math.floor(0xffffffff / max) * max;
  const buf = new Uint32Array(1);
  let x = 0;
  do {
    crypto.getRandomValues(buf);
    x = buf[0];
  } while (x >= limit);
  return x % max;
};

const generatePassword = () => {
  const upper = 'ABCDEFGHJKLMNPQRSTUVWXYZ';
  const lower = 'abcdefghijkmnopqrstuvwxyz';
  const digits = '23456789';
  const special = '!@#$%^&*()-_=+';
  const all = upper + lower + digits + special;
  const chars: string[] = [
    upper[randInt(upper.length)],
    lower[randInt(lower.length)],
    digits[randInt(digits.length)],
    special[randInt(special.length)],
  ];
  while (chars.length < 16) {
    chars.push(all[randInt(all.length)]);
  }
  for (let i = chars.length - 1; i > 0; i--) {
    const j = randInt(i + 1);
    const t = chars[i];
    chars[i] = chars[j];
    chars[j] = t;
  }
  passwordForm.value.password = chars.join('');
  notificationApi.showNotification('已生成 16 位强密码', 'success');
};

// 方法
const loadPasswords = async () => {
  loading.value = true;
  try {
    const response = await passwordApi.getPasswords();
    passwords.value = response || [];
  } catch (error) {
    console.error('加载密码失败:', error);
    notificationApi.showNotification('加载密码失败', 'error');
  } finally {
    loading.value = false;
  }
};

const handleSearch = () => {
  // 搜索逻辑已在计算属性中处理
};

const selectPassword = async (password: PasswordEntry) => {
  selectedPassword.value = password;
  // 后端在解锁会话内直接返回明文 password / notes
  decryptedPassword.value = password.password || '';
  showDetailDialog.value = true;
};

const copyPassword = async (password: PasswordEntry) => {
  try {
    // 后端在解锁会话内直接返回明文 password
    const decryptedPassword = password.password || '';
    await copyToClipboard(decryptedPassword);
    notificationApi.showNotification('密码已复制到剪贴板', 'success');
  } catch (error) {
    notificationApi.showNotification('复制失败', 'error');
  }
};

const editPassword = (password: PasswordEntry) => {
  editingPassword.value = password;
  passwordForm.value = {
    site_name: password.site_name,
    site_url: password.site_url,
    username: password.username,
    // 解锁会话内后端返回明文密码，预填以便直接保存；输入框为 password 类型（掩码显示）
    password: password.password || '',
    notes: password.notes || '',
    tags: password.tags,
    category: password.category,
    priority: password.priority,
  };
  showAddDialog.value = true;
};

const deletePassword = async (password: PasswordEntry) => {
  try {
    await ElMessageBox.confirm(
      `确定要删除密码 "${password.site_name}" 吗？此操作不可撤销。`,
      '确认删除',
      {
        confirmButtonText: '确定',
        cancelButtonText: '取消',
        type: 'warning',
      }
    );
    
    await passwordApi.deletePassword(password.id);
    passwords.value = passwords.value.filter(p => p.id !== password.id);
    notificationApi.showNotification('密码删除成功', 'success');
  } catch (error) {
    if (error !== 'cancel') {
      console.error('删除密码失败:', error);
      notificationApi.showNotification('删除密码失败', 'error');
    }
  }
};

const savePassword = async () => {
  try {
    await passwordFormRef.value?.validate();
    saving.value = true;
    
    const passwordData: NewPassword = {
      site_name: passwordForm.value.site_name,
      site_url: passwordForm.value.site_url,
      username: passwordForm.value.username,
      password: passwordForm.value.password,
    };
    
    if (editingPassword.value) {
      // 编辑模式
      const updateData: UpdatePassword = {
        site_name: passwordForm.value.site_name,
        site_url: passwordForm.value.site_url,
        username: passwordForm.value.username,
        password: passwordForm.value.password,
        notes: passwordForm.value.notes,
        tags: passwordForm.value.tags,
        category: passwordForm.value.category,
        priority: passwordForm.value.priority,
      };
      
      await passwordApi.updatePassword(editingPassword.value.id, updateData);
      notificationApi.showNotification('密码更新成功', 'success');
    } else {
      // 添加模式
      await passwordApi.addPassword(passwordData);
      notificationApi.showNotification('密码添加成功', 'success');
    }
    
    showAddDialog.value = false;
    editingPassword.value = null;
    await loadPasswords();
  } catch (error) {
    console.error('保存密码失败:', error);
    // 后端校验失败（如密码复用）的具体原因要透传给用户
    const message = error instanceof Error && error.message ? error.message : '保存密码失败';
    notificationApi.showNotification(message, 'error');
  } finally {
    saving.value = false;
  }
};

const copyToClipboard = async (text: string) => {
  try {
    await navigator.clipboard.writeText(text);
  } catch (error) {
    // 降级方案
    const textArea = document.createElement('textarea');
    textArea.value = text;
    document.body.appendChild(textArea);
    textArea.select();
    document.execCommand('copy');
    document.body.removeChild(textArea);
  }
};

const getCategoryName = (category: string) => {
  const categoryMap: Record<string, string> = {
    social: '社交',
    work: '工作',
    shopping: '购物',
    entertainment: '娱乐',
    other: '其他',
  };
  return categoryMap[category] || category;
};

const formatDate = (dateString: string) => {
  return new Date(dateString).toLocaleString();
};

// 生命周期
onMounted(() => {
  loadPasswords();
});
</script>

<style scoped>
.password-manager {
  padding: 20px;
}

.main-card {
  margin-bottom: 20px;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.search-bar {
  display: flex;
  gap: 10px;
  margin-bottom: 20px;
}

.search-input {
  flex: 1;
}

.password-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.password-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 15px;
  border: 1px solid #e4e7ed;
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.3s;
}

.password-item:hover {
  border-color: #409eff;
  box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
}

.password-info {
  flex: 1;
}

.site-info {
  margin-bottom: 8px;
}

.site-name {
  font-weight: bold;
  font-size: 16px;
  color: #303133;
}

.site-url {
  color: #909399;
  font-size: 14px;
}

.password-meta {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.username {
  color: #606266;
  font-size: 14px;
}

.tags {
  display: flex;
  gap: 5px;
}

.tag {
  font-size: 12px;
}

.password-actions {
  display: flex;
  gap: 5px;
}

.stats {
  margin-top: 20px;
}

.stat-card {
  text-align: center;
}

.stat-content {
  padding: 10px;
}

.stat-number {
  font-size: 24px;
  font-weight: bold;
  color: #409eff;
}

.stat-label {
  color: #909399;
  font-size: 14px;
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

.password-input {
  margin-top: 10px;
}

.link {
  color: #409eff;
  text-decoration: none;
}

.link:hover {
  text-decoration: underline;
}

.loading {
  padding: 20px;
}

.empty-state {
  text-align: center;
  padding: 40px;
}
</style>