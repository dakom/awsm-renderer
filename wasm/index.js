(function () {
    'use strict';

    function set_property(obj, name, value) { obj[name] = value; }

        function add_event(elem, name, f) {
            elem.addEventListener(name, f, {
                capture: true,
                once: false,
                passive: true
            });
        }

        function remove_event(elem, name, f) {
            elem.removeEventListener(name, f, true);
        }

    let wasm;

    const heap = new Array(32).fill(undefined);

    heap.push(undefined, null, true, false);

    function getObject(idx) { return heap[idx]; }

    let heap_next = heap.length;

    function dropObject(idx) {
        if (idx < 36) return;
        heap[idx] = heap_next;
        heap_next = idx;
    }

    function takeObject(idx) {
        const ret = getObject(idx);
        dropObject(idx);
        return ret;
    }

    function addHeapObject(obj) {
        if (heap_next === heap.length) heap.push(heap.length + 1);
        const idx = heap_next;
        heap_next = heap[idx];

        heap[idx] = obj;
        return idx;
    }

    let cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });

    cachedTextDecoder.decode();

    let cachegetUint8Memory0 = null;
    function getUint8Memory0() {
        if (cachegetUint8Memory0 === null || cachegetUint8Memory0.buffer !== wasm.memory.buffer) {
            cachegetUint8Memory0 = new Uint8Array(wasm.memory.buffer);
        }
        return cachegetUint8Memory0;
    }

    function getStringFromWasm0(ptr, len) {
        return cachedTextDecoder.decode(getUint8Memory0().subarray(ptr, ptr + len));
    }

    let WASM_VECTOR_LEN = 0;

    let cachedTextEncoder = new TextEncoder('utf-8');

    const encodeString = (typeof cachedTextEncoder.encodeInto === 'function'
        ? function (arg, view) {
        return cachedTextEncoder.encodeInto(arg, view);
    }
        : function (arg, view) {
        const buf = cachedTextEncoder.encode(arg);
        view.set(buf);
        return {
            read: arg.length,
            written: buf.length
        };
    });

    function passStringToWasm0(arg, malloc, realloc) {

        if (realloc === undefined) {
            const buf = cachedTextEncoder.encode(arg);
            const ptr = malloc(buf.length);
            getUint8Memory0().subarray(ptr, ptr + buf.length).set(buf);
            WASM_VECTOR_LEN = buf.length;
            return ptr;
        }

        let len = arg.length;
        let ptr = malloc(len);

        const mem = getUint8Memory0();

        let offset = 0;

        for (; offset < len; offset++) {
            const code = arg.charCodeAt(offset);
            if (code > 0x7F) break;
            mem[ptr + offset] = code;
        }

        if (offset !== len) {
            if (offset !== 0) {
                arg = arg.slice(offset);
            }
            ptr = realloc(ptr, len, len = offset + arg.length * 3);
            const view = getUint8Memory0().subarray(ptr + offset, ptr + len);
            const ret = encodeString(arg, view);

            offset += ret.written;
        }

        WASM_VECTOR_LEN = offset;
        return ptr;
    }

    function isLikeNone(x) {
        return x === undefined || x === null;
    }

    let cachegetInt32Memory0 = null;
    function getInt32Memory0() {
        if (cachegetInt32Memory0 === null || cachegetInt32Memory0.buffer !== wasm.memory.buffer) {
            cachegetInt32Memory0 = new Int32Array(wasm.memory.buffer);
        }
        return cachegetInt32Memory0;
    }

    let cachegetFloat64Memory0 = null;
    function getFloat64Memory0() {
        if (cachegetFloat64Memory0 === null || cachegetFloat64Memory0.buffer !== wasm.memory.buffer) {
            cachegetFloat64Memory0 = new Float64Array(wasm.memory.buffer);
        }
        return cachegetFloat64Memory0;
    }

    function debugString(val) {
        // primitive types
        const type = typeof val;
        if (type == 'number' || type == 'boolean' || val == null) {
            return  `${val}`;
        }
        if (type == 'string') {
            return `"${val}"`;
        }
        if (type == 'symbol') {
            const description = val.description;
            if (description == null) {
                return 'Symbol';
            } else {
                return `Symbol(${description})`;
            }
        }
        if (type == 'function') {
            const name = val.name;
            if (typeof name == 'string' && name.length > 0) {
                return `Function(${name})`;
            } else {
                return 'Function';
            }
        }
        // objects
        if (Array.isArray(val)) {
            const length = val.length;
            let debug = '[';
            if (length > 0) {
                debug += debugString(val[0]);
            }
            for(let i = 1; i < length; i++) {
                debug += ', ' + debugString(val[i]);
            }
            debug += ']';
            return debug;
        }
        // Test for built-in
        const builtInMatches = /\[object ([^\]]+)\]/.exec(toString.call(val));
        let className;
        if (builtInMatches.length > 1) {
            className = builtInMatches[1];
        } else {
            // Failed to match the standard '[object ClassName]'
            return toString.call(val);
        }
        if (className == 'Object') {
            // we're a user defined class or Object
            // JSON.stringify avoids problems with cycles, and is generally much
            // easier than looping through ownProperties of `val`.
            try {
                return 'Object(' + JSON.stringify(val) + ')';
            } catch (_) {
                return 'Object';
            }
        }
        // errors
        if (val instanceof Error) {
            return `${val.name}: ${val.message}\n${val.stack}`;
        }
        // TODO we could test for more things here, like `Set`s and `Map`s.
        return className;
    }

    function makeMutClosure(arg0, arg1, dtor, f) {
        const state = { a: arg0, b: arg1, cnt: 1, dtor };
        const real = (...args) => {
            // First up with a closure we increment the internal reference
            // count. This ensures that the Rust closure environment won't
            // be deallocated while we're invoking it.
            state.cnt++;
            const a = state.a;
            state.a = 0;
            try {
                return f(a, state.b, ...args);
            } finally {
                if (--state.cnt === 0) {
                    wasm.__wbindgen_export_2.get(state.dtor)(a, state.b);

                } else {
                    state.a = a;
                }
            }
        };
        real.original = state;

        return real;
    }

    let stack_pointer = 32;

    function addBorrowedObject(obj) {
        if (stack_pointer == 1) throw new Error('out of js stack');
        heap[--stack_pointer] = obj;
        return stack_pointer;
    }
    function __wbg_adapter_24(arg0, arg1, arg2) {
        try {
            wasm._dyn_core__ops__function__FnMut___A____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__h7dd451cc103ec27e(arg0, arg1, addBorrowedObject(arg2));
        } finally {
            heap[stack_pointer++] = undefined;
        }
    }

    function __wbg_adapter_27(arg0, arg1, arg2) {
        wasm._dyn_core__ops__function__FnMut__A____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__h1763342d47da7ba0(arg0, arg1, addHeapObject(arg2));
    }

    function __wbg_adapter_30(arg0, arg1, arg2) {
        wasm._dyn_core__ops__function__FnMut__A____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__h0e281d4eeadb413f(arg0, arg1, arg2);
    }

    function __wbg_adapter_33(arg0, arg1) {
        wasm._dyn_core__ops__function__FnMut_____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__hc1e4cffb53cc062b(arg0, arg1);
    }

    function __wbg_adapter_36(arg0, arg1, arg2) {
        wasm._dyn_core__ops__function__FnMut__A____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__hb0459477e117b6d5(arg0, arg1, addHeapObject(arg2));
    }

    function getCachedStringFromWasm0(ptr, len) {
        if (ptr === 0) {
            return getObject(len);
        } else {
            return getStringFromWasm0(ptr, len);
        }
    }

    function handleError(f) {
        return function () {
            try {
                return f.apply(this, arguments);

            } catch (e) {
                wasm.__wbindgen_exn_store(addHeapObject(e));
            }
        };
    }

    function getArrayU8FromWasm0(ptr, len) {
        return getUint8Memory0().subarray(ptr / 1, ptr / 1 + len);
    }

    let cachegetFloat32Memory0 = null;
    function getFloat32Memory0() {
        if (cachegetFloat32Memory0 === null || cachegetFloat32Memory0.buffer !== wasm.memory.buffer) {
            cachegetFloat32Memory0 = new Float32Array(wasm.memory.buffer);
        }
        return cachegetFloat32Memory0;
    }

    function getArrayF32FromWasm0(ptr, len) {
        return getFloat32Memory0().subarray(ptr / 4, ptr / 4 + len);
    }

    let cachegetUint32Memory0 = null;
    function getUint32Memory0() {
        if (cachegetUint32Memory0 === null || cachegetUint32Memory0.buffer !== wasm.memory.buffer) {
            cachegetUint32Memory0 = new Uint32Array(wasm.memory.buffer);
        }
        return cachegetUint32Memory0;
    }

    function getArrayU32FromWasm0(ptr, len) {
        return getUint32Memory0().subarray(ptr / 4, ptr / 4 + len);
    }

    async function load(module, imports) {
        if (typeof Response === 'function' && module instanceof Response) {

            if (typeof WebAssembly.instantiateStreaming === 'function') {
                try {
                    return await WebAssembly.instantiateStreaming(module, imports);

                } catch (e) {
                    if (module.headers.get('Content-Type') != 'application/wasm') {
                        console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);

                    } else {
                        throw e;
                    }
                }
            }

            const bytes = await module.arrayBuffer();
            return await WebAssembly.instantiate(bytes, imports);

        } else {

            const instance = await WebAssembly.instantiate(module, imports);

            if (instance instanceof WebAssembly.Instance) {
                return { instance, module };

            } else {
                return instance;
            }
        }
    }

    async function init(input) {
        if (typeof input === 'undefined') {
            input = (document.currentScript && document.currentScript.src || new URL('index.js', document.baseURI).href).replace(/\.js$/, '_bg.wasm');
        }
        const imports = {};
        imports.wbg = {};
        imports.wbg.__wbindgen_object_drop_ref = function(arg0) {
            takeObject(arg0);
        };
        imports.wbg.__wbg_closest_092d959de629c784 = handleError(function(arg0, arg1, arg2) {
            var v0 = getCachedStringFromWasm0(arg1, arg2);
            var ret = getObject(arg0).closest(v0);
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        });
        imports.wbg.__wbg_instanceof_HtmlElement_ed44c8f443dbd619 = function(arg0) {
            var ret = getObject(arg0) instanceof HTMLElement;
            return ret;
        };
        imports.wbg.__wbg_style_9b773f0fc441eddc = function(arg0) {
            var ret = getObject(arg0).style;
            return addHeapObject(ret);
        };
        imports.wbg.__wbg_classList_3cb76044e9478d02 = function(arg0) {
            var ret = getObject(arg0).classList;
            return addHeapObject(ret);
        };
        imports.wbg.__wbindgen_object_clone_ref = function(arg0) {
            var ret = getObject(arg0);
            return addHeapObject(ret);
        };
        imports.wbg.__wbg_addevent_ccc6ab519df88e22 = function(arg0, arg1, arg2, arg3) {
            var v0 = getCachedStringFromWasm0(arg1, arg2);
            add_event(getObject(arg0), v0, getObject(arg3));
        };
        imports.wbg.__wbg_instanceof_HtmlSelectElement_9c2391595418b732 = function(arg0) {
            var ret = getObject(arg0) instanceof HTMLSelectElement;
            return ret;
        };
        imports.wbg.__wbg_clientWidth_2c84a7ebc75dd4f0 = function(arg0) {
            var ret = getObject(arg0).clientWidth;
            return ret;
        };
        imports.wbg.__wbg_clientHeight_f56dfe03d1c42f3e = function(arg0) {
            var ret = getObject(arg0).clientHeight;
            return ret;
        };
        imports.wbg.__wbg_new_1fd0f2f67c61a072 = function() {
            var ret = new Object();
            return addHeapObject(ret);
        };
        imports.wbg.__wbg_new_8bedf78d5c7a3c29 = function(arg0, arg1) {
            var ret = new ResizeObserver(getObject(arg0), takeObject(arg1));
            return addHeapObject(ret);
        };
        imports.wbg.__wbg_observe_b347b2b5533632df = function(arg0, arg1) {
            getObject(arg0).observe(getObject(arg1));
        };
        imports.wbg.__wbg_disconnect_6084e81dc28c0960 = function(arg0) {
            getObject(arg0).disconnect();
        };
        imports.wbg.__wbindgen_string_new = function(arg0, arg1) {
            var ret = getStringFromWasm0(arg0, arg1);
            return addHeapObject(ret);
        };
        imports.wbg.__wbg_bindTexture_f3ab6393f75a763f = function(arg0, arg1, arg2) {
            getObject(arg0).bindTexture(arg1 >>> 0, getObject(arg2));
        };
        imports.wbg.__wbg_pixelStorei_8ec92e6e4c16982c = function(arg0, arg1, arg2) {
            getObject(arg0).pixelStorei(arg1 >>> 0, arg2);
        };
        imports.wbg.__wbg_texParameteri_6e7ba8c54bb639f2 = function(arg0, arg1, arg2, arg3) {
            getObject(arg0).texParameteri(arg1 >>> 0, arg2 >>> 0, arg3);
        };
        imports.wbg.__wbg_bindRenderbuffer_203ad50a30ad3c0d = function(arg0, arg1, arg2) {
            getObject(arg0).bindRenderbuffer(arg1 >>> 0, getObject(arg2));
        };
        imports.wbg.__wbg_bindFramebuffer_4a37c2a7678c0994 = function(arg0, arg1, arg2) {
            getObject(arg0).bindFramebuffer(arg1 >>> 0, getObject(arg2));
        };
        imports.wbg.__wbg_clearBufferfi_2e5fded6bfa7ae94 = function(arg0, arg1, arg2, arg3, arg4) {
            getObject(arg0).clearBufferfi(arg1 >>> 0, arg2, arg3, arg4);
        };
        imports.wbg.__wbg_clearBufferuiv_1e4a2a3e8513e3dd = function(arg0, arg1, arg2, arg3, arg4) {
            getObject(arg0).clearBufferuiv(arg1 >>> 0, arg2, getArrayU32FromWasm0(arg3, arg4));
        };
        imports.wbg.__wbindgen_cb_drop = function(arg0) {
            const obj = takeObject(arg0).original;
            if (obj.cnt-- == 1) {
                obj.a = 0;
                return true;
            }
            var ret = false;
            return ret;
        };
        imports.wbg.__wbg_clearBufferfv_276f9cc79778aa2c = function(arg0, arg1, arg2, arg3, arg4) {
            getObject(arg0).clearBufferfv(arg1 >>> 0, arg2, getArrayF32FromWasm0(arg3, arg4));
        };
        imports.wbg.__wbg_enable_f7d5513a12216046 = function(arg0, arg1) {
            getObject(arg0).enable(arg1 >>> 0);
        };
        imports.wbg.__wbg_blitFramebuffer_4b472bb45062bd6e = function(arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9, arg10) {
            getObject(arg0).blitFramebuffer(arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9 >>> 0, arg10 >>> 0);
        };
        imports.wbg.__wbg_depthMask_8e13d005f55547fa = function(arg0, arg1) {
            getObject(arg0).depthMask(arg1 !== 0);
        };
        imports.wbg.__wbg_blendFunc_8593e88646aa2829 = function(arg0, arg1, arg2) {
            getObject(arg0).blendFunc(arg1 >>> 0, arg2 >>> 0);
        };
        imports.wbg.__wbg_uniform4fv_1dbcc7a675cbf2d2 = function(arg0, arg1, arg2, arg3) {
            getObject(arg0).uniform4fv(getObject(arg1), getArrayF32FromWasm0(arg2, arg3));
        };
        imports.wbg.__wbg_uniformMatrix4fv_27bd1dff527241ff = function(arg0, arg1, arg2, arg3, arg4) {
            getObject(arg0).uniformMatrix4fv(getObject(arg1), arg2 !== 0, getArrayF32FromWasm0(arg3, arg4));
        };
        imports.wbg.__wbg_instanceof_MouseEvent_a4bbc498cded6110 = function(arg0) {
            var ret = getObject(arg0) instanceof MouseEvent;
            return ret;
        };
        imports.wbg.__wbg_getBoundingClientRect_505844bd8eb35668 = function(arg0) {
            var ret = getObject(arg0).getBoundingClientRect();
            return addHeapObject(ret);
        };
        imports.wbg.__wbg_clientX_3a14a1583294607f = function(arg0) {
            var ret = getObject(arg0).clientX;
            return ret;
        };
        imports.wbg.__wbg_clientY_4b4a322b80551002 = function(arg0) {
            var ret = getObject(arg0).clientY;
            return ret;
        };
        imports.wbg.__wbg_left_479514b443c100f4 = function(arg0) {
            var ret = getObject(arg0).left;
            return ret;
        };
        imports.wbg.__wbg_top_80a2533bf82e7a3e = function(arg0) {
            var ret = getObject(arg0).top;
            return ret;
        };
        imports.wbg.__wbg_height_aa24e3fef658c4a8 = function(arg0) {
            var ret = getObject(arg0).height;
            return ret;
        };
        imports.wbg.__wbg_new_68adb0d58759a4ed = function() {
            var ret = new Object();
            return addHeapObject(ret);
        };
        imports.wbg.__wbg_set_2e79e744454afade = function(arg0, arg1, arg2) {
            getObject(arg0)[takeObject(arg1)] = takeObject(arg2);
        };
        imports.wbg.__wbg_getContext_93be69215ea9dbbf = handleError(function(arg0, arg1, arg2, arg3) {
            var v0 = getCachedStringFromWasm0(arg1, arg2);
            var ret = getObject(arg0).getContext(v0, getObject(arg3));
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        });
        imports.wbg.__wbg_instanceof_WebGl2RenderingContext_9818b789249374d3 = function(arg0) {
            var ret = getObject(arg0) instanceof WebGL2RenderingContext;
            return ret;
        };
        imports.wbg.__wbg_canvas_15f8a286626410ff = function(arg0) {
            var ret = getObject(arg0).canvas;
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        };
        imports.wbg.__wbg_instanceof_HtmlCanvasElement_7bd3ee7838f11fc3 = function(arg0) {
            var ret = getObject(arg0) instanceof HTMLCanvasElement;
            return ret;
        };
        imports.wbg.__wbg_length_5451d14971418d5f = function(arg0) {
            var ret = getObject(arg0).length;
            return ret;
        };
        imports.wbg.__wbindgen_memory = function() {
            var ret = wasm.memory;
            return addHeapObject(ret);
        };
        imports.wbg.__wbg_buffer_3f12a1c608c6d04e = function(arg0) {
            var ret = getObject(arg0).buffer;
            return addHeapObject(ret);
        };
        imports.wbg.__wbg_new_2863e4d532e8dfb4 = function(arg0) {
            var ret = new Float32Array(getObject(arg0));
            return addHeapObject(ret);
        };
        imports.wbg.__wbg_set_424e78f4062c3790 = function(arg0, arg1, arg2) {
            getObject(arg0).set(getObject(arg1), arg2 >>> 0);
        };
        imports.wbg.__wbg_bufferData_b5889c60c7d61946 = function(arg0, arg1, arg2, arg3) {
            getObject(arg0).bufferData(arg1 >>> 0, getObject(arg2), arg3 >>> 0);
        };
        imports.wbg.__wbg_bufferData_8c572f7db0e55bdd = function(arg0, arg1, arg2, arg3, arg4) {
            getObject(arg0).bufferData(arg1 >>> 0, getArrayU8FromWasm0(arg2, arg3), arg4 >>> 0);
        };
        imports.wbg.__wbg_getUniformBlockIndex_13d69d04aaa79521 = function(arg0, arg1, arg2, arg3) {
            var v0 = getCachedStringFromWasm0(arg2, arg3);
            var ret = getObject(arg0).getUniformBlockIndex(getObject(arg1), v0);
            return ret;
        };
        imports.wbg.__wbg_uniformBlockBinding_e11d75a8b7073f0b = function(arg0, arg1, arg2, arg3) {
            getObject(arg0).uniformBlockBinding(getObject(arg1), arg2 >>> 0, arg3 >>> 0);
        };
        imports.wbg.__wbg_renderbufferStorageMultisample_0426ba65de4885db = function(arg0, arg1, arg2, arg3, arg4, arg5) {
            getObject(arg0).renderbufferStorageMultisample(arg1 >>> 0, arg2, arg3 >>> 0, arg4, arg5);
        };
        imports.wbg.__wbg_createFramebuffer_d01ac1b4f7c704e5 = function(arg0) {
            var ret = getObject(arg0).createFramebuffer();
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        };
        imports.wbg.__wbg_deleteFramebuffer_acd92acda81356e9 = function(arg0, arg1) {
            getObject(arg0).deleteFramebuffer(getObject(arg1));
        };
        imports.wbg.__wbg_checkFramebufferStatus_f742d2efafd5471f = function(arg0, arg1) {
            var ret = getObject(arg0).checkFramebufferStatus(arg1 >>> 0);
            return ret;
        };
        imports.wbg.__wbg_framebufferTexture2D_5b8575bda5aeceeb = function(arg0, arg1, arg2, arg3, arg4, arg5) {
            getObject(arg0).framebufferTexture2D(arg1 >>> 0, arg2 >>> 0, arg3 >>> 0, getObject(arg4), arg5);
        };
        imports.wbg.__wbg_framebufferRenderbuffer_5293612146bd869a = function(arg0, arg1, arg2, arg3, arg4) {
            getObject(arg0).framebufferRenderbuffer(arg1 >>> 0, arg2 >>> 0, arg3 >>> 0, getObject(arg4));
        };
        imports.wbg.__wbg_createRenderbuffer_c008c3bb50dfff9d = function(arg0) {
            var ret = getObject(arg0).createRenderbuffer();
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        };
        imports.wbg.__wbg_deleteRenderbuffer_b67ff9026d2be0fd = function(arg0, arg1) {
            getObject(arg0).deleteRenderbuffer(getObject(arg1));
        };
        imports.wbg.__wbg_renderbufferStorage_9853a44514453009 = function(arg0, arg1, arg2, arg3, arg4) {
            getObject(arg0).renderbufferStorage(arg1 >>> 0, arg2 >>> 0, arg3, arg4);
        };
        imports.wbg.__wbg_createShader_9378e5028efeddcf = function(arg0, arg1) {
            var ret = getObject(arg0).createShader(arg1 >>> 0);
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        };
        imports.wbg.__wbg_shaderSource_0b51ed30c2234a07 = function(arg0, arg1, arg2, arg3) {
            var v0 = getCachedStringFromWasm0(arg2, arg3);
            getObject(arg0).shaderSource(getObject(arg1), v0);
        };
        imports.wbg.__wbg_compileShader_3c4bd5d4666a9951 = function(arg0, arg1) {
            getObject(arg0).compileShader(getObject(arg1));
        };
        imports.wbg.__wbg_getShaderParameter_d3ad5fb12a1da258 = function(arg0, arg1, arg2) {
            var ret = getObject(arg0).getShaderParameter(getObject(arg1), arg2 >>> 0);
            return addHeapObject(ret);
        };
        imports.wbg.__wbindgen_boolean_get = function(arg0) {
            const v = getObject(arg0);
            var ret = typeof(v) === 'boolean' ? (v ? 1 : 0) : 2;
            return ret;
        };
        imports.wbg.__wbg_getShaderInfoLog_6e3d36e74e32aa2b = function(arg0, arg1, arg2) {
            var ret = getObject(arg1).getShaderInfoLog(getObject(arg2));
            var ptr0 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len0 = WASM_VECTOR_LEN;
            getInt32Memory0()[arg0 / 4 + 1] = len0;
            getInt32Memory0()[arg0 / 4 + 0] = ptr0;
        };
        imports.wbg.__wbg_createBuffer_a9e0a9167dc2f2b4 = function(arg0) {
            var ret = getObject(arg0).createBuffer();
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        };
        imports.wbg.__wbg_createTexture_151a385cd028c893 = function(arg0) {
            var ret = getObject(arg0).createTexture();
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        };
        imports.wbg.__wbg_deleteTexture_125ab82d8330e268 = function(arg0, arg1) {
            getObject(arg0).deleteTexture(getObject(arg1));
        };
        imports.wbg.__wbg_uniform1i_2cb54693e4c3bace = function(arg0, arg1, arg2) {
            getObject(arg0).uniform1i(getObject(arg1), arg2);
        };
        imports.wbg.__wbg_activeTexture_a756131b7b4547f3 = function(arg0, arg1) {
            getObject(arg0).activeTexture(arg1 >>> 0);
        };
        imports.wbg.__wbg_uniform2f_9983be9fcaad4947 = function(arg0, arg1, arg2, arg3) {
            getObject(arg0).uniform2f(getObject(arg1), arg2, arg3);
        };
        imports.wbg.__wbg_uniform3f_d2e6cef43185c5b7 = function(arg0, arg1, arg2, arg3, arg4) {
            getObject(arg0).uniform3f(getObject(arg1), arg2, arg3, arg4);
        };
        imports.wbg.__wbg_drawElements_4572c575d9e77ece = function(arg0, arg1, arg2, arg3, arg4) {
            getObject(arg0).drawElements(arg1 >>> 0, arg2, arg3 >>> 0, arg4);
        };
        imports.wbg.__wbg_drawArrays_5793555840ecaa0b = function(arg0, arg1, arg2, arg3) {
            getObject(arg0).drawArrays(arg1 >>> 0, arg2, arg3);
        };
        imports.wbg.__wbg_instanceof_WheelEvent_3b29517f8e243435 = function(arg0) {
            var ret = getObject(arg0) instanceof WheelEvent;
            return ret;
        };
        imports.wbg.__wbg_deltaMode_3db3c9c4bedf191d = function(arg0) {
            var ret = getObject(arg0).deltaMode;
            return ret;
        };
        imports.wbg.__wbg_instanceof_KeyboardEvent_6ede7b5da44a9d65 = function(arg0) {
            var ret = getObject(arg0) instanceof KeyboardEvent;
            return ret;
        };
        imports.wbg.__wbg_deltaX_5fac4f36a42e6ec9 = function(arg0) {
            var ret = getObject(arg0).deltaX;
            return ret;
        };
        imports.wbg.__wbg_deltaY_2722120e563d3160 = function(arg0) {
            var ret = getObject(arg0).deltaY;
            return ret;
        };
        imports.wbg.__wbg_deltaZ_0b2355692ca6ab12 = function(arg0) {
            var ret = getObject(arg0).deltaZ;
            return ret;
        };
        imports.wbg.__wbindgen_string_get = function(arg0, arg1) {
            const obj = getObject(arg1);
            var ret = typeof(obj) === 'string' ? obj : undefined;
            var ptr0 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len0 = WASM_VECTOR_LEN;
            getInt32Memory0()[arg0 / 4 + 1] = len0;
            getInt32Memory0()[arg0 / 4 + 0] = ptr0;
        };
        imports.wbg.__wbg_bindBuffer_2cb370d7ee8c8faa = function(arg0, arg1, arg2) {
            getObject(arg0).bindBuffer(arg1 >>> 0, getObject(arg2));
        };
        imports.wbg.__wbg_from_aee16bee83bf975b = function(arg0) {
            var ret = Array.from(getObject(arg0));
            return addHeapObject(ret);
        };
        imports.wbg.__wbg_drawBuffers_0b800e44adca1dbf = function(arg0, arg1) {
            getObject(arg0).drawBuffers(getObject(arg1));
        };
        imports.wbg.__wbg_useProgram_6b54e2f64672af62 = function(arg0, arg1) {
            getObject(arg0).useProgram(getObject(arg1));
        };
        imports.wbg.__wbg_createProgram_4823f8197c94860f = function(arg0) {
            var ret = getObject(arg0).createProgram();
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        };
        imports.wbg.__wbg_bindAttribLocation_e9acbae1a3a819fa = function(arg0, arg1, arg2, arg3, arg4) {
            var v0 = getCachedStringFromWasm0(arg3, arg4);
            getObject(arg0).bindAttribLocation(getObject(arg1), arg2 >>> 0, v0);
        };
        imports.wbg.__wbg_attachShader_386953a8caf97e31 = function(arg0, arg1, arg2) {
            getObject(arg0).attachShader(getObject(arg1), getObject(arg2));
        };
        imports.wbg.__wbg_linkProgram_be955380b2064b69 = function(arg0, arg1) {
            getObject(arg0).linkProgram(getObject(arg1));
        };
        imports.wbg.__wbg_getProgramParameter_7f66eafe63848c93 = function(arg0, arg1, arg2) {
            var ret = getObject(arg0).getProgramParameter(getObject(arg1), arg2 >>> 0);
            return addHeapObject(ret);
        };
        imports.wbg.__wbg_getProgramInfoLog_900722958284ce83 = function(arg0, arg1, arg2) {
            var ret = getObject(arg1).getProgramInfoLog(getObject(arg2));
            var ptr0 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len0 = WASM_VECTOR_LEN;
            getInt32Memory0()[arg0 / 4 + 1] = len0;
            getInt32Memory0()[arg0 / 4 + 0] = ptr0;
        };
        imports.wbg.__wbg_detachShader_0d8564c4e718d7fc = function(arg0, arg1, arg2) {
            getObject(arg0).detachShader(getObject(arg1), getObject(arg2));
        };
        imports.wbg.__wbg_deleteProgram_f19537f7a0ed5646 = function(arg0, arg1) {
            getObject(arg0).deleteProgram(getObject(arg1));
        };
        imports.wbg.__wbg_bindBufferBase_753c342f3b85b9a0 = function(arg0, arg1, arg2, arg3) {
            getObject(arg0).bindBufferBase(arg1 >>> 0, arg2 >>> 0, getObject(arg3));
        };
        imports.wbg.__wbg_getUniformLocation_02d298730d44dadc = function(arg0, arg1, arg2, arg3) {
            var v0 = getCachedStringFromWasm0(arg2, arg3);
            var ret = getObject(arg0).getUniformLocation(getObject(arg1), v0);
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        };
        imports.wbg.__wbg_uniform4ui_31bca485b700d482 = function(arg0, arg1, arg2, arg3, arg4, arg5) {
            getObject(arg0).uniform4ui(getObject(arg1), arg2 >>> 0, arg3 >>> 0, arg4 >>> 0, arg5 >>> 0);
        };
        imports.wbg.__wbg_bindVertexArray_569f8b5466293fb0 = function(arg0, arg1) {
            getObject(arg0).bindVertexArray(getObject(arg1));
        };
        imports.wbg.__wbg_getAttribLocation_713a1d120f1e32ba = function(arg0, arg1, arg2, arg3) {
            var v0 = getCachedStringFromWasm0(arg2, arg3);
            var ret = getObject(arg0).getAttribLocation(getObject(arg1), v0);
            return ret;
        };
        imports.wbg.__wbg_createVertexArray_1f35f6d163bbae13 = function(arg0) {
            var ret = getObject(arg0).createVertexArray();
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        };
        imports.wbg.__wbg_vertexAttribPointer_3eada76f7ac4fe76 = function(arg0, arg1, arg2, arg3, arg4, arg5, arg6) {
            getObject(arg0).vertexAttribPointer(arg1 >>> 0, arg2, arg3 >>> 0, arg4 !== 0, arg5, arg6);
        };
        imports.wbg.__wbg_vertexAttribIPointer_a9d88e5c5aa1da5c = function(arg0, arg1, arg2, arg3, arg4, arg5) {
            getObject(arg0).vertexAttribIPointer(arg1 >>> 0, arg2, arg3 >>> 0, arg4, arg5);
        };
        imports.wbg.__wbg_enableVertexAttribArray_3f2a29ade8fb65f9 = function(arg0, arg1) {
            getObject(arg0).enableVertexAttribArray(arg1 >>> 0);
        };
        imports.wbg.__wbindgen_number_get = function(arg0, arg1) {
            const obj = getObject(arg1);
            var ret = typeof(obj) === 'number' ? obj : undefined;
            getFloat64Memory0()[arg0 / 8 + 1] = isLikeNone(ret) ? 0 : ret;
            getInt32Memory0()[arg0 / 4 + 0] = !isLikeNone(ret);
        };
        imports.wbg.__wbg_texImage2D_ccb40c3682247024 = handleError(function(arg0, arg1, arg2, arg3, arg4, arg5, arg6) {
            getObject(arg0).texImage2D(arg1 >>> 0, arg2, arg3, arg4 >>> 0, arg5 >>> 0, getObject(arg6));
        });
        imports.wbg.__wbg_texImage2D_806180ca68ddf603 = handleError(function(arg0, arg1, arg2, arg3, arg4, arg5, arg6) {
            getObject(arg0).texImage2D(arg1 >>> 0, arg2, arg3, arg4 >>> 0, arg5 >>> 0, getObject(arg6));
        });
        imports.wbg.__wbg_texImage2D_dc3f868e66a269ad = handleError(function(arg0, arg1, arg2, arg3, arg4, arg5, arg6) {
            getObject(arg0).texImage2D(arg1 >>> 0, arg2, arg3, arg4 >>> 0, arg5 >>> 0, getObject(arg6));
        });
        imports.wbg.__wbg_texImage2D_8c4ef0458bccd3fd = handleError(function(arg0, arg1, arg2, arg3, arg4, arg5, arg6) {
            getObject(arg0).texImage2D(arg1 >>> 0, arg2, arg3, arg4 >>> 0, arg5 >>> 0, getObject(arg6));
        });
        imports.wbg.__wbg_texImage2D_c4bf7fe0d942bcbd = handleError(function(arg0, arg1, arg2, arg3, arg4, arg5, arg6) {
            getObject(arg0).texImage2D(arg1 >>> 0, arg2, arg3, arg4 >>> 0, arg5 >>> 0, getObject(arg6));
        });
        imports.wbg.__wbg_requestAnimationFrame_ef0e2294dc8b1088 = handleError(function(arg0, arg1) {
            var ret = getObject(arg0).requestAnimationFrame(getObject(arg1));
            return ret;
        });
        imports.wbg.__wbg_cancelAnimationFrame_60f9cf59ec1c0125 = handleError(function(arg0, arg1) {
            getObject(arg0).cancelAnimationFrame(arg1);
        });
        imports.wbg.__wbg_setwidth_1d0e975feecff3ef = function(arg0, arg1) {
            getObject(arg0).width = arg1 >>> 0;
        };
        imports.wbg.__wbg_setheight_7758ee3ff5c65474 = function(arg0, arg1) {
            getObject(arg0).height = arg1 >>> 0;
        };
        imports.wbg.__wbg_viewport_ec826bf788ce964f = function(arg0, arg1, arg2, arg3, arg4) {
            getObject(arg0).viewport(arg1, arg2, arg3, arg4);
        };
        imports.wbg.__wbg_removeevent_0f7fcdc74d4840b2 = function(arg0, arg1, arg2, arg3) {
            var v0 = getCachedStringFromWasm0(arg1, arg2);
            remove_event(getObject(arg0), v0, getObject(arg3));
        };
        imports.wbg.__wbg_value_48468fc865fd9e27 = function(arg0, arg1) {
            var ret = getObject(arg1).value;
            var ptr0 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len0 = WASM_VECTOR_LEN;
            getInt32Memory0()[arg0 / 4 + 1] = len0;
            getInt32Memory0()[arg0 / 4 + 0] = ptr0;
        };
        imports.wbg.__wbg_removeEventListener_4448b273b47328f8 = handleError(function(arg0, arg1, arg2, arg3, arg4) {
            var v0 = getCachedStringFromWasm0(arg1, arg2);
            getObject(arg0).removeEventListener(v0, getObject(arg3), arg4 !== 0);
        });
        imports.wbg.__wbg_disable_5c31195749c90c83 = function(arg0, arg1) {
            getObject(arg0).disable(arg1 >>> 0);
        };
        imports.wbg.__wbg_scissor_967dc192f6260c23 = function(arg0, arg1, arg2, arg3, arg4) {
            getObject(arg0).scissor(arg1, arg2, arg3, arg4);
        };
        imports.wbg.__wbg_readBuffer_dff50171de17536c = function(arg0, arg1) {
            getObject(arg0).readBuffer(arg1 >>> 0);
        };
        imports.wbg.__wbg_newwithbyteoffsetandlength_353b9b3e1eece05f = function(arg0, arg1, arg2) {
            var ret = new Uint16Array(getObject(arg0), arg1 >>> 0, arg2 >>> 0);
            return addHeapObject(ret);
        };
        imports.wbg.__wbg_readPixels_09140d80ea47e396 = handleError(function(arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7) {
            getObject(arg0).readPixels(arg1, arg2, arg3, arg4, arg5 >>> 0, arg6 >>> 0, getObject(arg7));
        });
        imports.wbg.__wbg_naturalWidth_47e71c9955ce982d = function(arg0) {
            var ret = getObject(arg0).naturalWidth;
            return ret;
        };
        imports.wbg.__wbg_naturalHeight_e310fb0a0aba9a9e = function(arg0) {
            var ret = getObject(arg0).naturalHeight;
            return ret;
        };
        imports.wbg.__wbg_new_1f2ded01c889cd38 = handleError(function() {
            var ret = new Image();
            return addHeapObject(ret);
        });
        imports.wbg.__wbg_location_c1e50a6e4c53d45c = function(arg0) {
            var ret = getObject(arg0).location;
            return addHeapObject(ret);
        };
        imports.wbg.__wbg_origin_be15168c886ad7ab = handleError(function(arg0, arg1) {
            var ret = getObject(arg1).origin;
            var ptr0 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len0 = WASM_VECTOR_LEN;
            getInt32Memory0()[arg0 / 4 + 1] = len0;
            getInt32Memory0()[arg0 / 4 + 0] = ptr0;
        });
        imports.wbg.__wbg_new_bcd64c6565e8d34f = handleError(function(arg0, arg1) {
            var v0 = getCachedStringFromWasm0(arg0, arg1);
            var ret = new URL(v0);
            return addHeapObject(ret);
        });
        imports.wbg.__wbg_origin_a4c92689076eab26 = function(arg0, arg1) {
            var ret = getObject(arg1).origin;
            var ptr0 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len0 = WASM_VECTOR_LEN;
            getInt32Memory0()[arg0 / 4 + 1] = len0;
            getInt32Memory0()[arg0 / 4 + 0] = ptr0;
        };
        imports.wbg.__wbg_setcrossOrigin_e1a35c32f0633f24 = function(arg0, arg1, arg2) {
            var v0 = getCachedStringFromWasm0(arg1, arg2);
            getObject(arg0).crossOrigin = v0;
        };
        imports.wbg.__wbg_setsrc_ca894d724570195d = function(arg0, arg1, arg2) {
            var v0 = getCachedStringFromWasm0(arg1, arg2);
            getObject(arg0).src = v0;
        };
        imports.wbg.__wbg_setonload_6fb79f39cb59de67 = function(arg0, arg1) {
            getObject(arg0).onload = getObject(arg1);
        };
        imports.wbg.__wbg_setonerror_bf29bf699c956162 = function(arg0, arg1) {
            getObject(arg0).onerror = getObject(arg1);
        };
        imports.wbg.__wbg_body_c8cb19d760637268 = function(arg0) {
            var ret = getObject(arg0).body;
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        };
        imports.wbg.__wbg_target_4bc4eb28204bcc44 = function(arg0) {
            var ret = getObject(arg0).target;
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        };
        imports.wbg.__wbg_document_c0366b39e4f4c89a = function(arg0) {
            var ret = getObject(arg0).document;
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        };
        imports.wbg.__wbg_createElement_99351c8bf0efac6e = handleError(function(arg0, arg1, arg2) {
            var v0 = getCachedStringFromWasm0(arg1, arg2);
            var ret = getObject(arg0).createElement(v0);
            return addHeapObject(ret);
        });
        imports.wbg.__wbg_createTextNode_cfdcc8da0d55d336 = function(arg0, arg1, arg2) {
            var v0 = getCachedStringFromWasm0(arg1, arg2);
            var ret = getObject(arg0).createTextNode(v0);
            return addHeapObject(ret);
        };
        imports.wbg.__wbg_setAttribute_e71b9086539f06a1 = handleError(function(arg0, arg1, arg2, arg3, arg4) {
            var v0 = getCachedStringFromWasm0(arg1, arg2);
            var v1 = getCachedStringFromWasm0(arg3, arg4);
            getObject(arg0).setAttribute(v0, v1);
        });
        imports.wbg.__wbg_add_81e93fbfffcb5553 = handleError(function(arg0, arg1, arg2) {
            var v0 = getCachedStringFromWasm0(arg1, arg2);
            getObject(arg0).add(v0);
        });
        imports.wbg.__wbg_removeProperty_dfad019c7011a427 = handleError(function(arg0, arg1, arg2, arg3) {
            var v0 = getCachedStringFromWasm0(arg2, arg3);
            var ret = getObject(arg1).removeProperty(v0);
            var ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len1 = WASM_VECTOR_LEN;
            getInt32Memory0()[arg0 / 4 + 1] = len1;
            getInt32Memory0()[arg0 / 4 + 0] = ptr1;
        });
        imports.wbg.__wbg_appendChild_7c45aeccd496f2a5 = handleError(function(arg0, arg1) {
            var ret = getObject(arg0).appendChild(getObject(arg1));
            return addHeapObject(ret);
        });
        imports.wbg.__wbg_setproperty_63cd02ca3f62dac4 = function(arg0, arg1, arg2, arg3) {
            var v0 = getCachedStringFromWasm0(arg1, arg2);
            set_property(getObject(arg0), v0, getObject(arg3));
        };
        imports.wbg.__wbg_getPropertyValue_51b3e4d3afc1266c = handleError(function(arg0, arg1, arg2, arg3) {
            var v0 = getCachedStringFromWasm0(arg2, arg3);
            var ret = getObject(arg1).getPropertyValue(v0);
            var ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len1 = WASM_VECTOR_LEN;
            getInt32Memory0()[arg0 / 4 + 1] = len1;
            getInt32Memory0()[arg0 / 4 + 0] = ptr1;
        });
        imports.wbg.__wbg_new_ba07d0daa0e4677e = function() {
            var ret = new Object();
            return addHeapObject(ret);
        };
        imports.wbg.__wbg_get_85e0a3b459845fe2 = handleError(function(arg0, arg1) {
            var ret = Reflect.get(getObject(arg0), getObject(arg1));
            return addHeapObject(ret);
        });
        imports.wbg.__wbg_now_7628760b7b640632 = function(arg0) {
            var ret = getObject(arg0).now();
            return ret;
        };
        imports.wbg.__wbg_self_6baf3a3aa7b63415 = handleError(function() {
            var ret = self.self;
            return addHeapObject(ret);
        });
        imports.wbg.__wbg_window_63fc4027b66c265b = handleError(function() {
            var ret = window.window;
            return addHeapObject(ret);
        });
        imports.wbg.__wbg_globalThis_513fb247e8e4e6d2 = handleError(function() {
            var ret = globalThis.globalThis;
            return addHeapObject(ret);
        });
        imports.wbg.__wbg_global_b87245cd886d7113 = handleError(function() {
            var ret = global.global;
            return addHeapObject(ret);
        });
        imports.wbg.__wbindgen_is_undefined = function(arg0) {
            var ret = getObject(arg0) === undefined;
            return ret;
        };
        imports.wbg.__wbg_newnoargs_7c6bd521992b4022 = function(arg0, arg1) {
            var v0 = getCachedStringFromWasm0(arg0, arg1);
            var ret = new Function(v0);
            return addHeapObject(ret);
        };
        imports.wbg.__wbg_call_951bd0c6d815d6f1 = handleError(function(arg0, arg1) {
            var ret = getObject(arg0).call(getObject(arg1));
            return addHeapObject(ret);
        });
        imports.wbg.__wbg_newwithbyteoffsetandlength_2016b902c412c87c = function(arg0, arg1, arg2) {
            var ret = new Uint32Array(getObject(arg0), arg1 >>> 0, arg2 >>> 0);
            return addHeapObject(ret);
        };
        imports.wbg.__wbg_newwithbyteoffsetandlength_9428545f18592c34 = function(arg0, arg1, arg2) {
            var ret = new Float32Array(getObject(arg0), arg1 >>> 0, arg2 >>> 0);
            return addHeapObject(ret);
        };
        imports.wbg.__wbg_set_9bdd413385146137 = handleError(function(arg0, arg1, arg2) {
            var ret = Reflect.set(getObject(arg0), getObject(arg1), getObject(arg2));
            return ret;
        });
        imports.wbg.__wbindgen_debug_string = function(arg0, arg1) {
            var ret = debugString(getObject(arg1));
            var ptr0 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len0 = WASM_VECTOR_LEN;
            getInt32Memory0()[arg0 / 4 + 1] = len0;
            getInt32Memory0()[arg0 / 4 + 0] = ptr0;
        };
        imports.wbg.__wbindgen_throw = function(arg0, arg1) {
            throw new Error(getStringFromWasm0(arg0, arg1));
        };
        imports.wbg.__wbg_then_dd3785597974798a = function(arg0, arg1) {
            var ret = getObject(arg0).then(getObject(arg1));
            return addHeapObject(ret);
        };
        imports.wbg.__wbg_resolve_6e61e640925a0db9 = function(arg0) {
            var ret = Promise.resolve(getObject(arg0));
            return addHeapObject(ret);
        };
        imports.wbg.__wbg_texImage2D_652cb43ecf938fbf = handleError(function(arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
            getObject(arg0).texImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8 >>> 0, getObject(arg9));
        });
        imports.wbg.__wbg_getParameter_be1e4b3ba2c0c339 = handleError(function(arg0, arg1) {
            var ret = getObject(arg0).getParameter(arg1 >>> 0);
            return addHeapObject(ret);
        });
        imports.wbg.__wbg_code_cbf76ad384ae1179 = function(arg0, arg1) {
            var ret = getObject(arg1).code;
            var ptr0 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len0 = WASM_VECTOR_LEN;
            getInt32Memory0()[arg0 / 4 + 1] = len0;
            getInt32Memory0()[arg0 / 4 + 0] = ptr0;
        };
        imports.wbg.__wbg_addEventListener_a422088e686210b5 = handleError(function(arg0, arg1, arg2, arg3, arg4) {
            var v0 = getCachedStringFromWasm0(arg1, arg2);
            getObject(arg0).addEventListener(v0, getObject(arg3), getObject(arg4));
        });
        imports.wbg.__wbg_setProperty_c4294d008ae89f52 = handleError(function(arg0, arg1, arg2, arg3, arg4, arg5, arg6) {
            var v0 = getCachedStringFromWasm0(arg1, arg2);
            var v1 = getCachedStringFromWasm0(arg3, arg4);
            var v2 = getCachedStringFromWasm0(arg5, arg6);
            getObject(arg0).setProperty(v0, v1, v2);
        });
        imports.wbg.__wbg_instanceof_Window_49f532f06a9786ee = function(arg0) {
            var ret = getObject(arg0) instanceof Window;
            return ret;
        };
        imports.wbg.__wbindgen_closure_wrapper323 = function(arg0, arg1, arg2) {
            var ret = makeMutClosure(arg0, arg1, 15, __wbg_adapter_24);
            return addHeapObject(ret);
        };
        imports.wbg.__wbindgen_closure_wrapper378 = function(arg0, arg1, arg2) {
            var ret = makeMutClosure(arg0, arg1, 24, __wbg_adapter_27);
            return addHeapObject(ret);
        };
        imports.wbg.__wbindgen_closure_wrapper381 = function(arg0, arg1, arg2) {
            var ret = makeMutClosure(arg0, arg1, 34, __wbg_adapter_30);
            return addHeapObject(ret);
        };
        imports.wbg.__wbindgen_closure_wrapper1072 = function(arg0, arg1, arg2) {
            var ret = makeMutClosure(arg0, arg1, 37, __wbg_adapter_33);
            return addHeapObject(ret);
        };
        imports.wbg.__wbindgen_closure_wrapper1073 = function(arg0, arg1, arg2) {
            var ret = makeMutClosure(arg0, arg1, 63, __wbg_adapter_36);
            return addHeapObject(ret);
        };

        if (typeof input === 'string' || (typeof Request === 'function' && input instanceof Request) || (typeof URL === 'function' && input instanceof URL)) {
            input = fetch(input);
        }

        const { instance, module } = await load(await input, imports);

        wasm = instance.exports;
        init.__wbindgen_wasm_module = module;
        wasm.__wbindgen_start();
        return wasm;
    }

    init("wasm/assets/demo-1e912337.wasm").catch(console.error);

}());
//# sourceMappingURL=index.js.map
