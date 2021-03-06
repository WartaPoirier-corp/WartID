const loginBoxList = document.querySelector('#login-box-list');
const loginBoxTemplate = document.querySelector('#login-box').content;

let i = 0;
for (const savedAccount of [{id: "ans", name: "ANS", pp: "https://cdn.discordapp.com/avatars/102076324067172352/b38901572554b317b2548bc04cdaa72d.webp?size=128"}]) {
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
