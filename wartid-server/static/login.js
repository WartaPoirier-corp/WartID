const loginBoxList = document.querySelector('#login-box-list');
const loginBoxTemplate = document.querySelector('#login-box').content;

let i = 0;
for (const savedAccount of []) {
    const clone = document.importNode(loginBoxTemplate, true);

    clone.querySelector('.login-box-pp').src = savedAccount.pp;
    clone.querySelector('.login-box-name').innerText = savedAccount.name;
    clone.querySelector('input[name="username"]').value = savedAccount.id;

    loginBoxList.appendChild(clone);
}

for (const loginBox of loginBoxList.children) {
    loginBox.onclick = () => {
        Array.from(loginBoxList.children).forEach(el => el.classList.remove('active'));
        loginBox.classList.add('active');
    }
}
