export async function projectRequest(path, method = 'GET', body, signal) {
  const response = await fetch(`/console/api/${path}`, {
    method, credentials: 'same-origin', cache: 'no-store', signal: signal || AbortSignal.timeout(45000),
    headers: body === undefined ? {} : {'Content-Type':'application/json'},
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  const value = await response.json();
  if (!response.ok) { const error = new Error(value.code || 'project_request_failed'); error.status = response.status; throw error; }
  return value;
}
export function projectName(project, text) { return project.name || text('未命名 Project', 'Untitled project'); }
export function projectError(code, text) {
  const messages = {
    forbidden: ['你没有管理此 Space 的权限。', 'You do not have permission to manage this Space.'],
    matrix_state_unavailable: ['暂时无法读取 Matrix 状态，请重试。', 'Matrix state is unavailable. Please retry.'],
    sign_in_required: ['登录已过期，请重新登录。', 'Your sign-in expired. Please sign in again.'],
    local_access_required: ['请从本机打开客户端以授权新账号。', 'Open the client locally to authorize a new account.'],
  };
  return messages[code] ? text(...messages[code]) : `${text('操作未完成', 'Operation failed')}: ${code}`;
}
